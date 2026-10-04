// Fork-specific compatibility coverage.
// SPDX-License-Identifier: AGPL-3.0-only

mod common;

use std::{
    collections::BTreeSet,
    io::Write,
    process::{Command, Output, Stdio},
};

use common::{Project, rule};
use erislint::{
    config::Config,
    runner::Plan,
    rust::{self, Span, Target, TargetKind},
};
use ra_ap_syntax::Edition;
use serde_json::json;

const SOURCE: &str = include_str!("fixtures/rust-compat/source.txt");
const GOLDEN: &[u8] = include_bytes!("fixtures/rust-compat/disk.json");

fn fixture() -> (Project, Config) {
    let project = Project::new();
    project.write("source.rs", SOURCE);
    let config = Config::load(&project.write(
        "erislint.json",
        include_bytes!("fixtures/rust-compat/config.json"),
    ))
    .unwrap();
    (project, config)
}

fn cli(project: &Project, args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_erislint"))
        .current_dir(project.root())
        .env_remove("jev_key")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(source) = stdin {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
    }
    child.wait_with_output().unwrap()
}

fn success(output: Output, expected: &[u8]) {
    assert!(output.status.success(), "{:?}", output);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, expected);
}

fn error<T>(result: anyhow::Result<T>) -> String {
    match result {
        Ok(_) => panic!("expected error"),
        Err(error) => format!("{error:#}"),
    }
}

#[test]
fn full_disk_and_unsaved_editor_requests_match_upstream_bytes() {
    let (project, _) = fixture();
    success(cli(&project, &["--dry-run"], None), GOLDEN);
    // A different saved buffer makes accidental disk reads observable.
    project.write("source.rs", "fn saved_buffer() {}\n");
    success(
        cli(
            &project,
            &["--dry-run", "--stdin-file", "source.rs"],
            Some(SOURCE),
        ),
        GOLDEN,
    );
    assert_eq!(
        std::fs::read_to_string(project.root().join("source.rs")).unwrap(),
        "fn saved_buffer() {}\n"
    );
}

#[test]
fn generated_schemas_match_committed_upstream_bytes() {
    let project = Project::new();
    for (kind, bytes) in [
        (
            "config",
            include_bytes!("../erislint.schema.json").as_slice(),
        ),
        (
            "rule",
            include_bytes!("../erislint-rule.schema.json").as_slice(),
        ),
    ] {
        success(cli(&project, &["--schema", kind], None), bytes);
    }
}

#[test]
fn old_public_imports_name_spans_and_full_ranges_remain_compatible() {
    let kinds = BTreeSet::from([TargetKind::Function, TargetKind::Impl, TargetKind::File]);
    let targets: Vec<Target> = rust::extract(SOURCE, Edition::Edition2024, &kinds).unwrap();
    let target = targets.iter().find(|t| t.name == "café").unwrap();
    let name: &Span = &target.span;
    assert_eq!(&SOURCE[name.start..name.end], "café");
    assert_eq!(
        &SOURCE[target.range.start..target.range.end],
        "/* 🦀 */ const fn café() -> u8 { 1 }"
    );
    assert_eq!((name.line, name.column, name.end_column), (18, 18, 22));
    assert!(target.range.start < name.start && target.range.end > name.end);
    for kind in [TargetKind::Impl, TargetKind::File] {
        let unnamed = targets.iter().find(|t| t.kind == kind).unwrap();
        assert_eq!(unnamed.span.start, unnamed.range.start);
        assert_eq!(unnamed.span.end, unnamed.range.end);
        assert_eq!(
            unnamed.name,
            if kind == TargetKind::File {
                "<file>"
            } else {
                "<impl>"
            }
        );
    }
}

#[test]
fn target_start_requires_exact_name_bytes_including_non_bmp_prefix() {
    let (project, _) = fixture();
    let start = SOURCE.find("café").unwrap();
    success(
        cli(
            &project,
            &[
                "--dry-run",
                "--stdin-file",
                "source.rs",
                "--target-start",
                &start.to_string(),
            ],
            Some(SOURCE),
        ),
        include_bytes!("fixtures/rust-compat/selected.json"),
    );
    // Declaration start, interior ASCII, interior UTF-8, name end, emoji interior,
    // and out-of-bounds offsets must fail rather than snap to a nearby target.
    for offset in [
        SOURCE.find("const fn").unwrap(),
        start + 1,
        start + 4,
        start + "café".len(),
        SOURCE.find('🦀').unwrap() + 1,
        SOURCE.len() + 1,
    ] {
        let output = cli(
            &project,
            &[
                "--dry-run",
                "--stdin-file",
                "source.rs",
                "--target-start",
                &offset.to_string(),
            ],
            Some(SOURCE),
        );
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            format!("erislint: no configured function rules at byte offset {offset}\n")
        );
    }
}

#[test]
fn disk_ignores_and_editor_filters_keep_distinct_behavior() {
    let project = Project::new();
    let good = project.write("good.rs", "fn good() {}");
    let ignored = project.write("ignored.rs", "fn ignored() {}");
    let filtered = project.write("filtered.rs", "fn filtered() {}");
    let target = project.write("target/build.rs", "fn build() {}");
    project.write(".erislintignore", "ignored.rs\n");
    project.write("other.txt", "not Rust syntax");
    let config = project.config(json!({"rules": [rule("quality")], "exclude": ["filtered.rs"]}));
    let plan = Plan::build(&config, &[]).unwrap();
    assert_eq!(plan.files, 1);
    assert_eq!(plan.evaluations[0].target, "good");
    assert_eq!(
        Plan::build(&config, &[ignored.clone(), good])
            .unwrap()
            .files,
        2
    );
    assert_eq!(
        Plan::from_source(&config, &ignored, "fn unsaved() {}")
            .unwrap()
            .evaluations[0]
            .target,
        "unsaved"
    );
    // Invalid Cargo metadata must not be consulted for filtered editor input.
    project.write("Cargo.toml", "invalid TOML [");
    for path in [filtered, target] {
        let plan = Plan::from_source(&config, &path, "not Rust syntax").unwrap();
        assert_eq!(plan.files, 1);
        assert!(plan.evaluations.is_empty());
    }
}

#[test]
fn editor_existence_containment_extension_and_edition_errors_keep_precedence() {
    let (project, config) = fixture();
    let missing = project.root().join("missing.txt");
    assert!(error(Plan::from_source(&config, &missing, "")).starts_with("cannot open input "));
    let outside = Project::new();
    let outside_file = outside.write("outside.txt", "");
    assert_eq!(
        error(Plan::from_source(&config, &outside_file, "")),
        "input is outside the configuration directory: prefix not found"
    );
    let text = project.write("existing.txt", "");
    project.write("Cargo.toml", "invalid TOML [");
    assert_eq!(
        error(Plan::from_source(&config, &text, "broken")),
        "editor input must be a Rust file"
    );
    assert!(
        error(Plan::from_source(
            &config,
            &project.root().join("source.rs"),
            "broken"
        ))
        .starts_with("invalid Cargo manifest ")
    );
    let output = cli(&project, &["--dry-run", "existing.txt"], None);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        output.stderr,
        b"erislint: no Rust source files matched the configured paths\n"
    );
}

#[test]
fn edition_selection_and_override_preserve_complete_requests() {
    let (project, _) = fixture();
    for edition in ["2018", "2021", "2024"] {
        project.write(
            "Cargo.toml",
            format!("[package]\nname = 'fixture'\nversion = '0.0.0'\nedition = '{edition}'\n"),
        );
        // Freeze identical output in editions that support async functions.
        success(cli(&project, &["--dry-run"], None), GOLDEN);
    }
    project.write(
        "Cargo.toml",
        "[package]\nname = 'fixture'\nversion = '0.0.0'\nedition = '2015'\n",
    );
    let output = cli(&project, &["--dry-run"], None);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        output.stderr,
        b"erislint: cannot parse source.rs: Rust syntax errors:\n20:20: expected an item\n"
    );
    project.write(
        "Cargo.toml",
        "[workspace]\n[workspace.package]\nedition = '2021'\n",
    );
    project.write(
        "member/Cargo.toml",
        "[package]\nname = 'member'\nversion = '0.0.0'\nedition.workspace = true\n",
    );
    let path = project.write("member/lib.rs", "fn gen() {}");
    let config = Config::load(&project.root().join("erislint.json")).unwrap();
    assert_eq!(rust::edition_for(&path).unwrap(), Edition::Edition2021);
    assert!(
        !Plan::build(&config, std::slice::from_ref(&path))
            .unwrap()
            .evaluations
            .is_empty()
    );
    let mut value: serde_json::Value =
        serde_json::from_slice(include_bytes!("fixtures/rust-compat/config.json")).unwrap();
    value["edition"] = json!("2024");
    let config = project.config(value);
    assert!(
        error(Plan::build(&config, std::slice::from_ref(&path))).contains("Rust syntax errors")
    );
    assert!(error(Plan::from_source(&config, &path, "fn gen() {}")).contains("Rust syntax errors"));
    project.write(
        "member/Cargo.toml",
        "[package]\nname = 'member'\nversion = '0.0.0'\n",
    );
    assert_eq!(rust::edition_for(&path).unwrap(), Edition::Edition2015);
}

#[test]
fn malformed_source_has_identical_disk_and_editor_failure() {
    let (project, _) = fixture();
    let broken = "/* 🦀 */ fn broken( {";
    project.write("source.rs", broken);
    let disk = cli(&project, &["--dry-run"], None);
    let editor = cli(
        &project,
        &["--dry-run", "--stdin-file", "source.rs"],
        Some(broken),
    );
    assert_eq!(disk.status.code(), Some(2));
    assert_eq!(editor.status.code(), Some(2));
    assert!(disk.stdout.is_empty() && editor.stdout.is_empty());
    assert_eq!(disk.stderr, editor.stderr);
    assert!(
        String::from_utf8(disk.stderr)
            .unwrap()
            .starts_with("erislint: cannot parse source.rs: Rust syntax errors:\n")
    );
}
