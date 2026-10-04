// Fork-specific x86 assembly contracts. Synthetic source only.
// SPDX-License-Identifier: AGPL-3.0-only
mod common;
use common::{Project, rule};
use erislint::config::{
    Config, Language,
    assembly::{Profile, UNCERTAINTY_DESCRIPTION},
};
use serde_json::{Value, json};

fn asm_rule(id: &str, kind: &str) -> Value {
    let mut r = rule(id);
    r["where"] = json!({"language":"assembly","profile":"x86-gas-att32","kind":kind});
    r["question"]["criteria"]
        .as_object_mut()
        .unwrap()
        .remove("unknown");
    r["question"]["criteria"]["insufficient_context"] = json!(UNCERTAINTY_DESCRIPTION);
    r
}
fn config(rules: Vec<Value>) -> Value {
    json!({"version":3,"include":["**/*.s","**/*.S"],"assembly_sources":[{"files":["**/*.s","**/*.S"],"profile":"x86-gas-att32","preprocessing":"none","slash_mode":"gas-default"}],"rules":rules})
}
fn error<T>(result: anyhow::Result<T>) -> String {
    match result {
        Ok(_) => panic!("expected error"),
        Err(e) => format!("{e:#}"),
    }
}
#[test]
fn v3_assembly_requires_explicit_implemented_profile_options_and_pairing() {
    let p = Project::new();
    let valid = config(vec![asm_rule("a", "file")]);
    let c = p.config(valid.clone());
    assert_eq!(c.assembly_sources[0].options.profile, Profile::X86GasAtt32);
    for field in ["profile", "preprocessing", "slash_mode", "files"] {
        let mut v = valid.clone();
        v["assembly_sources"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            Config::load(&p.json("erislint.json", &v)).is_err(),
            "{field}"
        );
    }
    for profile in ["mos-llvm-c64", "unknown"] {
        let mut v = valid.clone();
        v["assembly_sources"][0]["profile"] = json!(profile);
        let e = error(Config::load(&p.json("erislint.json", &v)));
        if profile == "mos-llvm-c64" {
            assert!(e.contains("not implemented"));
        }
    }
    for field in ["assembly_sources", "rules"] {
        let mut v = valid.clone();
        v.as_object_mut().unwrap().remove(field);
        assert!(Config::load(&p.json("erislint.json", &v)).is_err());
    }
    for kind in ["function", "class", "struct"] {
        assert!(
            Config::load(&p.json("erislint.json", &config(vec![asm_rule("a", kind)]))).is_err()
        );
    }
}
#[test]
fn uncertainty_is_required_only_for_assembly_and_policy_references_are_rejected() {
    let p = Project::new();
    for bad in [
        json!({}),
        json!({"choice":"insufficient_context"}),
        json!({"choice":"bad","any":[{"probability":{"choice":"insufficient_context","min":0.1}}]}),
    ] {
        let mut r = asm_rule("a", "file");
        r["diagnostics"][0]["when"] = bad;
        assert!(Config::load(&p.json("erislint.json", &config(vec![r]))).is_err());
    }
    for missing in [true, false] {
        let mut r = asm_rule("a", "file");
        if missing {
            r["question"]["criteria"]
                .as_object_mut()
                .unwrap()
                .remove("insufficient_context");
        } else {
            r["question"]["criteria"]["insufficient_context"] = json!("wrong");
        }
        assert!(Config::load(&p.json("erislint.json", &config(vec![r]))).is_err());
    }
    for version in [1, 2, 3] {
        p.config(json!({"version":version,"rules":[rule("legacy")]}));
    }
}
#[test]
fn legacy_documents_and_rule_imports_reject_new_fields_even_null() {
    let p = Project::new();
    for version in [1, 2] {
        let mut v = json!({"version":version,"rules":[rule("r")]});
        v["assembly_sources"] = Value::Null;
        assert!(Config::load(&p.json("erislint.json", &v)).is_err());
        let mut r = rule("r");
        r["where"]["profile"] = Value::Null;
        p.json("rules.json", &r);
        assert!(
            Config::load(&p.json(
                "erislint.json",
                &json!({"version":version,"rule_files":["rules.json"]})
            ))
            .is_err()
        );
        assert!(
            Config::load(&p.json(
                "erislint.json",
                &json!({"version":3,"rule_files":[{"path":"rules.json","version":version}]})
            ))
            .is_err()
        );
    }
}
#[test]
fn inheritance_preserves_document_and_rule_provenance_in_both_directions() {
    let p = Project::new();
    p.json("older/rules.json", &rule("r"));
    for version in [1, 2] {
        p.json(
            "older/base.json",
            &json!({"version":version,"rule_files":["rules.json"]}),
        );
        let c = p.config(json!({"version":3,"extends":["older/base.json"]}));
        assert_eq!(c.rules["r"].origin.version, version);
        assert!(c.rules["r"].origin.path.ends_with("older/rules.json"));
        p.json("new.json", &json!({"version":3,"rules":[rule("r")]}));
        assert!(
            error(Config::load(&p.json(
                "erislint.json",
                &json!({"version":version,"extends":["new.json"]})
            )))
            .contains("cannot extend version 3")
        );
    }
    for (child, base) in [(1, 2), (2, 1), (3, 3)] {
        p.json("base.json", &json!({"version":base,"rules":[rule("r")]}));
        assert_eq!(
            p.config(json!({"version":child,"extends":["base.json"]}))
                .rules["r"]
                .origin
                .version,
            base
        );
    }
    p.json("base.json", &json!({"version":2,"rules":[rule("r")]}));
    let mut v = config(vec![asm_rule("r", "assembly_region")]);
    v["extends"] = json!(["base.json"]);
    let c = p.config(v);
    assert_eq!(c.rules["r"].origin.version, 3);
    assert_eq!(
        c.rules["r"].definition.r#where.language(),
        Language::Assembly
    );
}
#[test]
fn v3_imports_validate_explicit_version_not_schema_hint() {
    let p = Project::new();
    let mut py = rule("p");
    py["where"]["language"] = json!("python");
    p.json("old.json", &py);
    let c=p.config(json!({"version":3,"python_files":["**/*.py"],"rule_files":[{"path":"old.json","version":2}]}));
    assert_eq!(c.rules["p"].origin.version, 2);
    let mut assembly = asm_rule("a", "file");
    assembly["$schema"] = json!("erislint-rule-v2.schema.json");
    p.json("new.json", &assembly);
    let mut v = config(vec![]);
    v["rule_files"] = json!(["new.json"]);
    p.config(v.clone());
    for version in [1, 2, 4] {
        v["rule_files"] = json!([{"path":"new.json","version":version}]);
        assert!(Config::load(&p.json("erislint.json", &v)).is_err());
    }
}

#[test]
fn shared_source_records_preserve_crlf_unicode_and_empty_file_spans() {
    use erislint::assembly::text::Text;
    let source = "  # erislint-region-begin r\r\n\t.byte \"é\"\r\n  # erislint-region-end r\r\n";
    let text = Text::new(source);
    let span = text.span(37, 39);
    assert_eq!((span.line, span.column, span.end_column), (2, 9, 10));
    assert_eq!(text.record("region", 29, 42).source, "\t.byte \"é\"\r\n");
    assert_eq!(text.record("file", 0, source.len()).source, source);
    let empty = Text::new("").span(0, 0);
    assert_eq!((empty.line, empty.column, empty.end), (1, 1, 0));
}

use erislint::{
    assembly::lexer::tokenize,
    config::assembly::{Options, Preprocessing, SlashMode},
};
fn options() -> Options {
    Options {
        profile: Profile::X86GasAtt32,
        preprocessing: Preprocessing::None,
        slash_mode: SlashMode::GasDefault,
    }
}
#[test]
fn x86_lexical_records_cover_every_original_byte_without_interpreting_operands() {
    let source = ".code32\r\n1: movl $1,4(%eax,%ecx,4); jmp 1b # é\r\n.ascii \"#;/*not comments*/\\\"\"\r\n.byte 'A, '\\n, '#\r\nname/*x*/suffix\n";
    let p = tokenize(source, options()).unwrap();
    assert_eq!(
        p.records.iter().map(|r| r.source).collect::<String>(),
        source
    );
    let mut offset = 0;
    for r in &p.records {
        assert_eq!(r.span.start, offset);
        assert_eq!(&source[r.span.start..r.span.end], r.source);
        offset = r.span.end;
    }
    assert_eq!(offset, source.len());
    assert_eq!(p.statements.iter().filter(|s| s.kind == "label").count(), 1);
    assert!(
        p.records
            .iter()
            .any(|r| r.kind == "character" && r.source == "'#")
    );
    assert!(
        p.statements
            .iter()
            .any(|r| r.kind == "opaque_statement" && r.source == "jmp 1b")
    );
}
#[test]
fn x86_literals_and_slash_modes_reject_unsupported_forms_without_fallback() {
    for source in [
        ".byte 'A'",
        ".byte 'é",
        ".byte '",
        ".byte '\n",
        ".byte '\r\n",
        ".byte '\\q",
        ".ascii \"x\\q\"",
        ".ascii \"x\n\"",
        ".ascii \"x\\\ny\"",
        ".ascii \"missing",
        "/*missing",
        "*/",
        "nop\rnext",
    ] {
        assert!(tokenize(source, options()).is_err(), "{source:?}");
    }
    let mut divide = options();
    divide.slash_mode = SlashMode::Divide;
    assert!(tokenize("nop // comment", divide).is_err());
    assert_eq!(
        tokenize("nop / comment", options())
            .unwrap()
            .records
            .last()
            .unwrap()
            .kind,
        "comment"
    );
    assert!(
        tokenize("value = 4/2\n", divide)
            .unwrap()
            .records
            .iter()
            .any(|r| r.source == "/" && r.kind == "punctuation")
    );
    for source in [
        ".byte 'A",
        ".byte '\\n",
        ".byte '\\'",
        ".byte '\\\\",
        ".ascii \"é\\\"\"",
        "movl $1, %eax /* note */\n",
    ] {
        tokenize(source, options()).unwrap();
    }
}
#[test]
fn cpp_continuations_precede_macros_state_controls_literals_and_markers() {
    let source = "#define BODY \\\n .macro hidden \\\n .intel_syntax noprefix \\\n #APP \\\n # erislint-region-begin fake\n.code32\n";
    let mut cpp = options();
    cpp.preprocessing = Preprocessing::CppUnexpanded;
    for input in [source.to_owned(), source.replace('\n', "\r\n")] {
        let p = tokenize(&input, cpp).unwrap();
        assert_eq!(p.records.iter().filter(|r| r.kind == "cpp").count(), 1);
        assert!(p.regions.is_empty());
        assert_eq!(
            p.records.iter().map(|r| r.source).collect::<String>(),
            input
        );
    }
    assert!(error(tokenize(source, options())).contains("preprocessing_mode_required"));
    for source in [
        "#define X \\",
        "#define X \\\n",
        "#define X 1\n.intel_syntax noprefix\n",
    ] {
        assert!(tokenize(source, cpp).is_err());
    }
    tokenize("/* #define x \\\n .intel_syntax */\n", options()).unwrap();
}
#[test]
fn structural_states_are_balanced_and_unsupported_controls_fail_even_when_inactive() {
    let source = ".macro outer p\n.irp n,a,b\n.irpc c,ab\n.rept 2\n.byte \\p\n.endr\n.endr\n.endr\n.endm\n.if 0\nunknown_call\n.else\nother\n.endif\n";
    let p = tokenize(source, options()).unwrap();
    assert!(p.dependencies.iter().any(|r| r.kind == "macro_body"));
    assert_eq!(
        p.dependencies
            .iter()
            .filter(|r| r.kind == "repetition_body")
            .count(),
        3
    );
    for bad in [
        ".endm",
        ".macro",
        ".macro m\n",
        ".rept 2\n.endm",
        ".macro a\n.macro b\n.endm\n.endm",
        ".ifc a,b\n.endif",
        ".if 0\n.else\n.else\n.endif",
    ] {
        assert!(tokenize(bad, options()).is_err(), "{bad}");
    }
    for directive in [
        ".code16",
        ".code64",
        ".intel_syntax",
        ".altmacro",
        ".noaltmacro",
        ".cpu",
        ".syntax",
    ] {
        for source in [
            format!("{directive}\n"),
            format!(".macro m\n{directive}\n.endm\n"),
            format!(".if 0\n{directive}\n.endif\n"),
        ] {
            assert!(tokenize(&source, options()).is_err());
        }
    }
    for control in ["#APP", "  #NO_APP\r\n", ".macro m\n#APP\n.endm\n"] {
        assert!(tokenize(control, options()).is_err());
    }
    tokenize("#APP extra\n.ascii \"#NO_APP\"\n# note #APP\n", options()).unwrap();
}
#[test]
fn region_markers_have_exact_ranges_and_ignore_inert_comment_or_macro_text() {
    let source = "  # erislint-region-begin r\r\n\t.byte \"é\"\r\n  # erislint-region-end r\r\n";
    let p = tokenize(source, options()).unwrap();
    let r = &p.regions[0];
    assert_eq!((r.body.span.start, r.body.span.end), (29, 42));
    assert_eq!((r.name_span.start, r.name_span.end), (26, 27));
    assert_eq!(
        (r.begin_comment.span.start, r.begin_comment.span.end),
        (2, 27)
    );
    assert_eq!((r.end_line.span.start, r.end_line.span.end), (42, 69));
    let inert = "/*\n# erislint-region-begin fake\n*/\n.ascii \"# erislint-region-end fake\"\n.macro m\n# erislint-region-begin inside\n.endm\nnop # erislint-region-end inline\n#erislint-region-begin nospace\n";
    assert!(tokenize(inert, options()).unwrap().regions.is_empty());
    let eof = source.trim_end_matches("\r\n");
    assert_eq!(
        tokenize(eof, options()).unwrap().regions[0]
            .end_line
            .span
            .end,
        67
    );
}
#[test]
fn malformed_empty_nested_and_duplicate_regions_are_operational_errors() {
    for name in ["", "1bad", "é", "bad/name", "two names", &"a".repeat(65)] {
        let source = format!("# erislint-region-begin {name}\nnop\n# erislint-region-end {name}\n");
        assert!(tokenize(&source, options()).is_err(), "{name}");
    }
    for body in ["", " \t\r\n", "\u{2003}\n"] {
        let source = format!("# erislint-region-begin r\n{body}# erislint-region-end r\n");
        assert!(tokenize(&source, options()).is_err());
    }
    for source in [
        "# erislint-region-begin r",
        "# erislint-region-end r\n",
        "# erislint-region-begin r\nnop\n# erislint-region-end x\n",
        "# erislint-region-begin r\n# erislint-region-begin x\n",
    ] {
        assert!(tokenize(source, options()).is_err());
    }
    let good = "# erislint-region-begin _r.1-a\n# comment only\n# erislint-region-end _r.1-a\n";
    tokenize(good, options()).unwrap();
    assert!(tokenize(&good.repeat(2), options()).is_err());
}

use erislint::{
    assembly,
    config::InputContext,
    policy::Level,
    runner::{Plan, diagnostics},
    source::TargetKind,
};
use std::{collections::BTreeSet, process::Command};
#[test]
fn legacy_cli_error_bytes_match_pre_v3_binary() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/legacy-errors.json")).unwrap();
    for c in fixture["cases"].as_array().unwrap() {
        let p = Project::new();
        p.write("erislint.json", c["config_text"].as_str().unwrap());
        for (name, source) in c["files"].as_object().unwrap() {
            p.write(name, source.as_str().unwrap());
        }
        let args: Vec<_> = c["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect();
        let out = Command::new(env!("CARGO_BIN_EXE_erislint"))
            .current_dir(p.root())
            .env_remove("jev_key")
            .args(["--config", p.root().join("erislint.json").to_str().unwrap()])
            .args(args)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert_eq!(
            String::from_utf8(out.stderr)
                .unwrap()
                .replace(p.root().to_str().unwrap(), "<ROOT>"),
            c["stderr"],
            "{}",
            c["name"]
        );
    }
}
#[test]
fn essential_dependencies_survive_all_context_modes_and_file_bytes_are_complete() {
    let source = ".set FLAG, unknown\n.macro helper\nnop\n.endm\n.if FLAG\n# erislint-region-begin r\nhelper\n# erislint-region-end r\n.endif\n";
    let ts = assembly::extract(
        source,
        options(),
        &BTreeSet::from([TargetKind::File, TargetKind::AssemblyRegion]),
    )
    .unwrap();
    assert_eq!(ts.len(), 2);
    let base = ts[1].input(InputContext::Target, source);
    assert_eq!(base["source"], "helper\n");
    assert!(
        base["analysis"]["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["kind"] == "macro_body")
    );
    for context in [
        InputContext::Target,
        InputContext::Enclosing,
        InputContext::File,
    ] {
        assert_eq!(ts[1].input(context, source)["analysis"], base["analysis"]);
    }
    for source in ["", " \t\r\n"] {
        let t = assembly::extract(source, options(), &BTreeSet::from([TargetKind::File])).unwrap();
        assert_eq!(t[0].range.end, source.len());
        assert_eq!(t[0].input(InputContext::Target, source)["source"], source);
    }
}
#[test]
fn active_region_rules_fail_without_markers_but_explicit_off_skips() {
    let p = Project::new();
    let path = p.write("a.s", "nop\n");
    let mut v = config(vec![
        asm_rule("file", "file"),
        asm_rule("region", "assembly_region"),
    ]);
    let c = p.config(v.clone());
    assert!(error(Plan::build(&c, &[])).contains("no assembly regions"));
    v["overrides"] = json!([{"files":["**"],"rules":{"region":"off"}}]);
    let c = p.config(v.clone());
    assert_eq!(Plan::build(&c, &[]).unwrap().evaluations.len(), 1);
    v["overrides"][0]["rules"]["file"] = json!("off");
    let c = p.config(v);
    assert!(
        Plan::from_source(&c, &path, ".intel_syntax")
            .unwrap()
            .evaluations
            .is_empty()
    );
}
#[test]
fn assembly_uncertainty_is_fixed_warning_and_substantive_policies_are_unchanged() {
    let p = Project::new();
    p.write("a.s", "nop\n");
    for setting in ["warn", "error"] {
        let mut v = config(vec![asm_rule("a", "file")]);
        v["overrides"] = json!([{"files":["**"],"rules":{"a":setting}}]);
        let c = p.config(v);
        let plan = Plan::build(&c, &[]).unwrap();
        assert!(
            plan.evaluations[0].request.questions["a"]
                .choices()
                .contains_key("insufficient_context")
        );
        for (choice, probabilities) in [
            (
                "insufficient_context",
                json!({"good":0.1,"bad":0.1,"insufficient_context":0.8}),
            ),
            (
                "bad",
                json!({"good":0.1,"bad":0.8,"insufficient_context":0.1}),
            ),
        ] {
            let response=serde_json::from_value(json!({"model":"mock","answers":{"a":{"type":"choice","choice":choice,"confidence":0.99,"probabilities":probabilities}}})).unwrap();
            let ds = diagnostics(&c, &plan.evaluations[0], response).unwrap();
            assert_eq!(ds.len(), 1);
            if choice == "insufficient_context" {
                assert_eq!(ds[0].level, Level::Warn);
                assert_eq!(ds[0].message, erislint::config::assembly::INCONCLUSIVE);
            } else {
                assert_eq!(
                    ds[0].level,
                    if setting == "warn" {
                        Level::Warn
                    } else {
                        Level::Error
                    }
                );
            }
        }
    }
}
#[test]
fn mixed_v3_cache_selection_filtering_and_unsaved_sources_are_separate() {
    let p = Project::new();
    p.write(
        "Cargo.toml",
        "[package]\nname='mixed'\nversion='0.1.0'\nedition='2024'\n",
    );
    let paths = [
        p.write("a.rs", "fn a() {}"),
        p.write("b.c", "int b(void){return 0;}"),
        p.write("c.py", "def c(): pass"),
        p.write("d.s", "value = 4/2\n"),
        p.write("e.S", "#define X 1\nnop\n"),
    ];
    let mut c = rule("c");
    c["where"]["language"] = json!("c");
    let mut py = rule("p");
    py["where"]["language"] = json!("python");
    let mut v = config(vec![rule("r"), c, py, asm_rule("a", "file")]);
    v["include"] = json!(["**"]);
    v["c_files"] = json!(["**/*.c"]);
    v["python_files"] = json!(["**/*.py"]);
    v["assembly_sources"] = json!([{"files":["*.s"],"profile":"x86-gas-att32","preprocessing":"none","slash_mode":"divide"},{"files":["*.S"],"profile":"x86-gas-att32","preprocessing":"cpp-unexpanded","slash_mode":"gas-default"}]);
    let c = p.config(v.clone());
    let expected = serde_json::to_value(Plan::build(&c, &[]).unwrap()).unwrap();
    assert_eq!(expected["evaluations"].as_array().unwrap().len(), 5);
    let reversed: Vec<_> = paths
        .iter()
        .rev()
        .cloned()
        .chain([paths[0].clone()])
        .collect();
    assert_eq!(
        serde_json::to_value(Plan::build(&c, &reversed).unwrap()).unwrap(),
        expected
    );
    let snapshot = Plan::from_source(&c, &paths[3], ".byte 'A\n").unwrap();
    assert_eq!(
        snapshot.evaluations[0].request.state["source"],
        ".byte 'A\n"
    );
    assert_eq!(std::fs::read_to_string(&paths[3]).unwrap(), "value = 4/2\n");
    v["assembly_sources"][0]["files"] = json!(["**"]);
    v["include"] = json!(["d.s"]);
    let c = p.config(v.clone());
    assert_eq!(Plan::build(&c, &[]).unwrap().files, 1);
    assert!(
        Plan::from_source(&c, &paths[0], "bad")
            .unwrap()
            .evaluations
            .is_empty()
    );
    v["include"] = json!(["**"]);
    let c = p.config(v);
    assert!(Plan::build(&c, &[]).is_err());
}
