// Fork-specific C and mixed-language contracts.
// SPDX-License-Identifier: AGPL-3.0-only

mod common;

use common::{Project, response, rule};
use erislint::{
    c,
    config::{Config, InputContext},
    runner::{Plan, diagnostics},
    source::TargetKind,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::Write,
    process::{Command, Stdio},
};

fn c_rule(id: &str, kind: &str) -> Value {
    let mut rule = rule(id);
    rule["where"]["language"] = json!("c");
    rule["where"]["kind"] = json!(kind);
    rule["where"].as_object_mut().unwrap().remove("has_body");
    rule
}

fn config(rules: Vec<Value>) -> Value {
    json!({"version": 2, "include": ["**"], "c_files": ["**/*.c", "**/*.h"], "rules": rules})
}

fn error<T>(result: anyhow::Result<T>) -> String {
    match result {
        Ok(_) => panic!("expected an error"),
        Err(error) => format!("{error:#}"),
    }
}

#[test]
fn preserves_comments_preprocessor_branches_and_unicode_spans() {
    let source = "// file 🦀\r\n#include \"missing.h\"\r\n#define VALUE 3\r\n#if FLAG\r\n/* lead 🦀 */ int café(int x) { /* body */ return x + VALUE; }\r\n#else\r\nint other(void) { return 0; }\r\n#endif\r\n";
    let targets = c::extract(
        source,
        &BTreeSet::from([TargetKind::Function, TargetKind::File]),
    )
    .unwrap();
    assert_eq!(targets.len(), 3);
    let file = &targets[0];
    assert_eq!(file.input(InputContext::Target, source)["source"], source);
    let function = targets.iter().find(|target| target.name == "café").unwrap();
    assert_eq!(&source[function.span.start..function.span.end], "café");
    assert_eq!(
        (
            function.span.line,
            function.span.column,
            function.span.end_column
        ),
        (5, 18, 22)
    );
    assert_eq!(
        &source[function.range.start..function.range.end],
        "int café(int x) { /* body */ return x + VALUE; }"
    );
    let state = function.input(InputContext::Enclosing, source);
    assert_eq!(state["language"], "c");
    assert_eq!(state["parameters"], "(int x)");
    assert_eq!(state["body"], "{ /* body */ return x + VALUE; }");
    assert_eq!(state["analysis"]["completeness"], "incomplete");
    assert!(
        state["comments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["source"] == "/* lead 🦀 */")
    );
    assert!(
        state["preprocessor"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["kind"] == "preproc_include")
    );
    assert_eq!(state["context"]["enclosing"][0]["kind"], "preproc_if");
    let other = targets
        .iter()
        .find(|target| target.name == "other")
        .unwrap()
        .input(InputContext::Enclosing, source);
    assert_eq!(other["context"]["enclosing"][1]["kind"], "preproc_else");
    assert_eq!(
        function.input(InputContext::File, source)["context"]["file"],
        source
    );
}

#[test]
fn distinguishes_prototypes_pointer_variables_and_function_return_declarators() {
    let source = "int first(void), second(int); int (*callback)(int); int *factory(void); int (*choose(void))(int); typedef int Handler(int); int run(int (*f)(int)) { return f(1); }";
    let targets = c::extract(source, &BTreeSet::from([TargetKind::Function])).unwrap();
    assert_eq!(
        targets
            .iter()
            .map(|target| target.name.as_str())
            .collect::<Vec<_>>(),
        ["first", "second", "factory", "choose", "run"]
    );
    assert_eq!(
        targets
            .iter()
            .map(|target| target.has_body)
            .collect::<Vec<_>>(),
        [false, false, false, false, true]
    );
    assert_eq!(
        targets[1].input(InputContext::Target, source)["declarator"],
        "second(int)"
    );
    assert_eq!(
        targets[2].input(InputContext::Target, source)["declarator"],
        "*factory(void)"
    );
    assert_eq!(
        targets[3].input(InputContext::Target, source)["parameters"],
        "(void)"
    );
    assert_eq!(
        targets[4].input(InputContext::Target, source)["parameters"],
        "(int (*f)(int))"
    );
}

#[test]
fn malformed_cpp_and_empty_function_extractions_are_operational_errors() {
    let functions = BTreeSet::from([TargetKind::Function]);
    for source in [
        "int broken( {",
        "int f(void) { return 1;",
        "namespace demo { int f(); }",
        "extern \"C\" { int f(void); }",
    ] {
        assert!(c::extract(source, &functions).is_err(), "{source}");
        assert!(
            c::extract(source, &BTreeSet::from([TargetKind::File])).is_err(),
            "file-only {source}"
        );
    }
    assert!(error(c::extract("int global;", &functions)).contains("no supported C targets"));
    assert!(c::extract("int global;", &BTreeSet::from([TargetKind::File])).is_ok());
    assert!(
        error(c::extract(
            "struct S { int a; };",
            &BTreeSet::from([TargetKind::Struct])
        ))
        .contains("only function and file")
    );
}

#[test]
fn mixed_directory_cache_and_explicit_order_are_language_independent() {
    for (c_name, rust_name) in [("a.c", "z.rs"), ("z.c", "a.rs")] {
        let project = Project::new();
        project.write(
            "Cargo.toml",
            "[package]\nname = 'sample'\nversion = '0.0.0'\nedition = '2021'\n",
        );
        let c_path = project.write(c_name, "int c_function(void) { return 1; }");
        let rust_path = project.write(rust_name, "fn gen() {}");
        let config = project.config(config(vec![
            rule("rust-only"),
            c_rule("c-only", "function"),
        ]));
        let plan = Plan::build(&config, &[]).unwrap();
        assert_eq!(plan.files, 2);
        assert_eq!(plan.evaluations.len(), 2);
        let expected = serde_json::to_vec(&plan).unwrap();
        for paths in [
            vec![c_path.clone(), rust_path.clone()],
            vec![rust_path.clone(), c_path.clone(), c_path.clone()],
        ] {
            assert_eq!(
                serde_json::to_vec(&Plan::build(&config, &paths).unwrap()).unwrap(),
                expected
            );
        }
        for evaluation in &plan.evaluations {
            let c = evaluation.request.state["language"] == "c";
            assert_eq!(
                evaluation
                    .request
                    .questions
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                [if c { "c-only" } else { "rust-only" }]
            );
            assert_eq!(evaluation.target, if c { "c_function" } else { "gen" });
        }
    }
}

#[test]
fn c_only_never_reads_cargo_and_headers_require_explicit_c_mode() {
    let project = Project::new();
    project.write("Cargo.toml", "invalid TOML [");
    let header = project.write("api.h", "int declared(void);");
    let source = project.write("main.c", "int defined(void) { return 1; }");
    let c_config = project.config(config(vec![c_rule("c", "function")]));
    assert_eq!(Plan::build(&c_config, &[]).unwrap().evaluations.len(), 2);
    assert_eq!(
        Plan::from_source(&c_config, &header, "int unsaved(void);")
            .unwrap()
            .evaluations[0]
            .target,
        "unsaved"
    );
    let mut value = config(vec![c_rule("c", "function")]);
    value["c_files"] = json!(["**/*.c"]);
    let c_config = project.config(value);
    assert_eq!(Plan::build(&c_config, &[]).unwrap().files, 1);
    assert!(Plan::from_source(&c_config, &header, "int f(void);").is_err());
    let rust_config = project.config(json!({"include": ["**"], "rules": [rule("rust")]}));
    assert_eq!(
        error(Plan::build(&rust_config, &[source, header])),
        "no Rust source files matched the configured paths"
    );
}

#[test]
fn v2_validation_rejects_ambiguous_or_unsupported_language_configuration() {
    let project = Project::new();
    let mut version1 = config(vec![c_rule("c", "function")]);
    version1["version"] = json!(1);
    let mut missing_selection = config(vec![c_rule("c", "function")]);
    missing_selection.as_object_mut().unwrap().remove("c_files");
    let mut missing_rule = config(vec![rule("rust")]);
    missing_rule["rules"] = json!([rule("rust")]);
    let mut invalid_kind = config(vec![c_rule("c", "struct")]);
    invalid_kind["rules"][0]["where"]
        .as_object_mut()
        .unwrap()
        .remove("has_body");
    let mut unknown = config(vec![c_rule("c", "function")]);
    unknown["rules"][0]["where"]["language"] = json!("cpp");
    let mut empty = config(vec![c_rule("c", "function")]);
    empty["c_files"] = json!([]);
    for value in [
        version1,
        missing_selection,
        missing_rule,
        invalid_kind,
        unknown,
        empty,
    ] {
        assert!(Config::load(&project.json("erislint.json", &value)).is_err());
    }
    let path = project.write("input.cpp", "int f() { return 1; }");
    let mut value = config(vec![c_rule("c", "function")]);
    value["c_files"] = json!(["**/*.cpp"]);
    let config = project.config(value);
    assert!(
        error(Plan::build(&config, std::slice::from_ref(&path))).contains("unsupported extension")
    );
    assert!(
        error(Plan::from_source(&config, &path, "int f() {}")).contains("unsupported extension")
    );
}

#[test]
fn c_inheritance_rules_and_filters_preserve_explicit_intent() {
    let project = Project::new();
    project.write("one.c", "int one(void) { return 1; }");
    let two = project.write("two.c", "int two(void) { return 2; }");
    project.json("base.json", &config(vec![c_rule("c", "function")]));
    let child = project.json(
        "child.json",
        &json!({"version":2,"extends":["base.json"],"c_files":["one.c"]}),
    );
    assert_eq!(
        Plan::build(&Config::load(&child).unwrap(), &[])
            .unwrap()
            .files,
        1
    );
    project.json("rules.json", &c_rule("c", "function"));
    let v1 = project.json("v1.json", &json!({"rule_files":["rules.json"]}));
    assert!(error(Config::load(&v1)).contains("version 2"));
    let child = project.json(
        "child.json",
        &json!({"version":2,"extends":["v1.json"],"c_files":["**/*.c"]}),
    );
    assert!(Config::load(&child).is_err());
    let mut value = config(vec![c_rule("c", "function")]);
    value["overrides"] = json!([{"files":["two.c"],"rules":{"c":"off"}}]);
    let config = project.config(value);
    assert_eq!(Plan::build(&config, &[]).unwrap().evaluations.len(), 1);
    assert!(
        Plan::from_source(&config, &two, "malformed (")
            .unwrap()
            .evaluations
            .is_empty()
    );
}

#[test]
fn c_uses_shared_diagnostics_and_no_match_is_not_silent_success() {
    let project = Project::new();
    let source = project.write("sample.c", "int sum(int a, int b) { return a + b; }");
    let config = project.config(config(vec![c_rule("c-quality", "function")]));
    let plan = Plan::build(&config, &[]).unwrap();
    let response = serde_json::from_value(response("c-quality", 0.9)).unwrap();
    let diagnostics = diagnostics(&config, &plan.evaluations[0], response).unwrap();
    assert_eq!(diagnostics[0].message, "Simplify sum");
    assert_eq!(diagnostics[0].location.span.start, "int ".len());
    assert_eq!(diagnostics[0].location.file.to_str(), Some("sample.c"));
    project.write("sample.c", "int global;");
    assert!(
        error(Plan::build(&config, std::slice::from_ref(&source)))
            .contains("no supported C targets")
    );
}

#[test]
fn cli_c_snapshot_selects_exact_name_without_writing_or_credentials() {
    let project = Project::new();
    project.write("sample.c", "int saved(void);\n");
    project.config(config(vec![c_rule("c", "function")]));
    let source = "/* 🦀 */ int café(void) { return 1; }\r\n";
    let start = source.find("café").unwrap();
    for offset in [start, start + 4] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_erislint"))
            .current_dir(project.root())
            .env_remove("jev_key")
            .args([
                "--dry-run",
                "--stdin-file",
                "sample.c",
                "--target-start",
                &offset.to_string(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        if offset == start {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let report: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(report["evaluations"][0]["target"], "café");
            assert_eq!(
                report["evaluations"][0]["request"]["state"]["language"],
                "c"
            );
        } else {
            assert_eq!(output.status.code(), Some(2));
        }
    }
    assert_eq!(
        std::fs::read_to_string(project.root().join("sample.c")).unwrap(),
        "int saved(void);\n"
    );
}

#[test]
fn version_one_rejects_new_fields_even_when_null() {
    let project = Project::new();
    let mut value = json!({"rules":[rule("rust")], "c_files":null});
    assert!(Config::load(&project.json("erislint.json", &value)).is_err());
    value.as_object_mut().unwrap().remove("c_files");
    value["rules"][0]["where"]["language"] = Value::Null;
    assert!(Config::load(&project.json("erislint.json", &value)).is_err());
    project.json("rules.json", &value["rules"]);
    assert!(
        Config::load(&project.json("erislint.json", &json!({"rule_files":["rules.json"]})))
            .is_err()
    );
}

#[test]
fn version_one_cannot_inherit_c_enablement_from_version_two() {
    let project = Project::new();
    project.json("base.json", &config(vec![c_rule("c", "file")]));
    let child = project.json(
        "erislint.json",
        &json!({"version":1,"extends":["base.json"]}),
    );
    assert!(error(Config::load(&child)).contains("selected config version 2"));
}

#[test]
fn v2_schema_commands_match_separate_checked_in_schemas() {
    for (kind, expected) in [
        (
            "config-v2",
            include_bytes!("../erislint-v2.schema.json").as_slice(),
        ),
        (
            "rule-v2",
            include_bytes!("../erislint-rule-v2.schema.json").as_slice(),
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_erislint"))
            .env_remove("jev_key")
            .args(["--schema", kind])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, expected);
    }
}

#[test]
fn complete_c_example_requests_match_frozen_bytes() {
    let project = Project::new();
    project.write("sample.c", include_str!("../examples/c/sample.c"));
    project.write("sample.h", include_str!("../examples/c/sample.h"));
    project.write("erislint.json", include_str!("../examples/c/erislint.json"));
    let output = Command::new(env!("CARGO_BIN_EXE_erislint"))
        .current_dir(project.root())
        .env_remove("jev_key")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, include_bytes!("fixtures/c-example.json"));
}

#[test]
fn old_style_parameter_declarations_are_not_function_targets() {
    let source = "int invoke(callback)\nint callback(void);\n{ return callback(); }";
    let targets = c::extract(source, &BTreeSet::from([TargetKind::Function])).unwrap();
    assert_eq!(
        targets
            .iter()
            .map(|target| target.name.as_str())
            .collect::<Vec<_>>(),
        ["invoke"]
    );
    let state = targets[0].input(InputContext::Target, source);
    assert_eq!(state["source"], source);
    assert_eq!(state["parameters"], "(callback)");
    assert_eq!(state["analysis"]["completeness"], "incomplete");
}

#[test]
fn parameter_context_does_not_hide_file_or_block_function_declarations() {
    let project = Project::new();
    let source = "int callback(void);\nint invoke(callback)\nint callback(void);\n{ int local(void); { extern int nested(void); } return callback(); }\nint modern(int parameter(void)) { return parameter(); }\n";
    project.write("old_style.c", source);
    let config = project.config(config(vec![c_rule("c", "function")]));
    let plan = Plan::build(&config, &[]).unwrap();
    assert_eq!(
        plan.evaluations
            .iter()
            .map(|evaluation| evaluation.target.as_str())
            .collect::<Vec<_>>(),
        ["callback", "invoke", "local", "nested", "modern"]
    );
    assert_eq!(
        plan.evaluations[0].location.span.start,
        source.find("callback").unwrap()
    );
    assert_eq!(
        plan.evaluations[1].request.state["source"],
        "int invoke(callback)\nint callback(void);\n{ int local(void); { extern int nested(void); } return callback(); }"
    );
    assert_eq!(plan.evaluations[2].request.state["body"], Value::Null);
    assert_eq!(
        plan.evaluations[3].request.state["context"]["enclosing"][0]["kind"],
        "function_definition"
    );
}
