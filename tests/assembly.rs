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
