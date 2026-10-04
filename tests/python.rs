// Fork-specific Python contracts.
// SPDX-License-Identifier: AGPL-3.0-only
mod common;
use erislint::{config::InputContext, python, source::TargetKind};
use std::collections::BTreeSet;

#[test]
fn preserves_decorated_async_methods_nesting_and_metadata() {
    let source = "# café 🦀\r\n\"\"\"module docs\"\"\"\r\n@decorate(flag=True)\r\nclass Café(Base, metaclass=Meta):\r\n    # before docs\r\n    r\"class docs\"\r\n    @staticmethod\r\n    async def méthode(x: list[int] = [1]) -> str:\r\n        \"\"\"method docs\"\"\"\r\n        def nested(y: int):\r\n            return y\r\n        return str(x)\r\n";
    let targets = python::extract(
        source,
        &BTreeSet::from([TargetKind::File, TargetKind::Class, TargetKind::Function]),
    )
    .unwrap();
    assert_eq!(
        targets.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
        ["<file>", "Café", "méthode", "nested"]
    );
    assert_eq!(
        targets[0].input(InputContext::Target, source)["source"],
        source
    );
    let class = targets[1].input(InputContext::Target, source);
    assert_eq!(class["bases"]["source"], "(Base, metaclass=Meta)");
    assert_eq!(class["docstring"]["source"], "r\"class docs\"");
    let method = &targets[2];
    let state = method.input(InputContext::Enclosing, source);
    assert_eq!(&source[method.span.start..method.span.end], "méthode");
    assert_eq!(
        (method.span.line, method.span.column, method.span.end_column),
        (8, 15, 22)
    );
    assert!(source[method.range.start..method.range.end].starts_with("@staticmethod\r\n"));
    assert_eq!(state["async"], true);
    assert_eq!(state["method"], true);
    assert_eq!(state["parameters"]["source"], "(x: list[int] = [1])");
    assert_eq!(state["return_annotation"]["source"], "str");
    assert_eq!(state["docstring"]["source"], "\"\"\"method docs\"\"\"");
    assert_eq!(state["context"]["enclosing"].as_array().unwrap().len(), 1);
    assert_eq!(state["comments"].as_array().unwrap().len(), 2);
    let nested = targets[3].input(InputContext::File, source);
    assert_eq!(nested["method"], false);
    assert_eq!(nested["context"]["file"], source);
    assert_eq!(nested["context"]["enclosing"].as_array().unwrap().len(), 2);
    assert_eq!(nested["analysis"]["completeness"], "incomplete");
}

#[test]
fn files_cover_whitespace_comments_and_empty_source() {
    for source in ["", " \t\r\n \t", "\n\n# comment\n", "\n\nvalue = 1\n"] {
        let t = python::extract(source, &BTreeSet::from([TargetKind::File])).unwrap();
        assert_eq!((t[0].range.start, t[0].range.end), (0, source.len()));
        assert_eq!((t[0].span.start, t[0].span.end), (0, source.len()));
        assert_eq!(t[0].input(InputContext::Target, source)["source"], source);
    }
}

#[test]
fn rejects_parse_errors_python2_and_missing_targets() {
    for source in [
        "def broken(:\n pass",
        "class :\n pass",
        "print 'legacy'",
        "exec code",
        "x = `y`",
        "def f():\n",
    ] {
        assert!(
            python::extract(source, &BTreeSet::from([TargetKind::File])).is_err(),
            "{source}"
        );
    }
    assert!(python::extract("value = lambda: 1", &BTreeSet::from([TargetKind::Function])).is_err());
    assert!(python::extract("class A: pass", &BTreeSet::from([TargetKind::Struct])).is_err());
}

#[test]
fn docstrings_are_raw_literals_not_interpolated_or_bytes_expressions() {
    for expression in ["f'dynamic {value}'", "b'bytes'", "'later'"] {
        let prefix = if expression == "'later'" { "x=1\n" } else { "" };
        let source = format!("{prefix}{expression}\n");
        let t = python::extract(&source, &BTreeSet::from([TargetKind::File])).unwrap();
        assert!(t[0].input(InputContext::Target, &source)["docstring"].is_null());
    }
    let source = "'one' 'two'\n";
    let t = python::extract(source, &BTreeSet::from([TargetKind::File])).unwrap();
    assert_eq!(
        t[0].input(InputContext::Target, source)["docstring"]["source"],
        "'one' 'two'"
    );
}

#[test]
fn preserves_type_parameters_and_match_syntax_without_execution() {
    let source = "import missing_module\n@missing_decorator()\ndef identity[T](value: T) -> T:\n    match value:\n        case _: return value\nclass Box[T]: pass\n";
    let t = python::extract(
        source,
        &BTreeSet::from([TargetKind::File, TargetKind::Function, TargetKind::Class]),
    )
    .unwrap();
    assert_eq!(t.len(), 3);
    assert_eq!(
        t[1].input(InputContext::Target, source)["type_parameters"]["source"],
        "[T]"
    );
}

use common::{Project, response, rule};
use erislint::{
    config::Config,
    runner::{Plan, diagnostics},
};
use serde_json::{Value, json};

fn python_rule(id: &str, kind: &str) -> Value {
    let mut r = rule(id);
    r["where"] = json!({"language":"python","kind":kind});
    r
}
fn config(rules: Vec<Value>) -> Value {
    json!({"version":2,"include":["**"],"python_files":["**/*.py"],"rules":rules})
}
fn error<T>(r: anyhow::Result<T>) -> String {
    match r {
        Ok(_) => panic!("expected failure"),
        Err(e) => format!("{e:#}"),
    }
}

#[test]
fn requires_explicit_python_selection_and_language_in_version_two() {
    let p = Project::new();
    for value in [
        json!({"version":1,"python_files":null,"rules":[rule("r")]}),
        json!({"version":1,"python_files":["**/*.py"],"rules":[python_rule("r","function")]}),
        json!({"version":2,"rules":[python_rule("r","function")]}),
        json!({"version":2,"python_files":["**/*.py"],"rules":[rule("r")]}),
        json!({"version":2,"python_files":[],"rules":[python_rule("r","function")]}),
        config(vec![python_rule("r", "struct")]),
        json!({"version":1,"rules":[{"id":"r","where":{"kind":"class"}}]}),
        {
            let mut r = rule("r");
            r["where"] = json!({"kind":"class"});
            json!({"version":2,"rules":[r]})
        },
    ] {
        assert!(
            Config::load(&p.json("erislint.json", &value)).is_err(),
            "{value}"
        );
    }
    p.write("sample.py", "def visible(): pass\n");
    let c = p.config(config(vec![python_rule("r", "class")]));
    assert!(error(Plan::build(&c, &[])).contains("no supported Python targets"));
    let c = p.config(json!({"version":2,"edition":"2024","include":["**"],"rules":[rule("r")]}));
    assert!(error(Plan::build(&c, &[])).contains("no Rust source"));
}

#[test]
fn rejects_python_inherited_by_version_one_and_external_legacy_rules() {
    let p = Project::new();
    p.json("base.json", &config(vec![python_rule("r", "function")]));
    assert!(
        error(Config::load(&p.json(
            "erislint.json",
            &json!({"version":1,"extends":["base.json"]})
        )))
        .contains("inherited Python")
    );
    p.json("rules.json", &python_rule("r", "function"));
    assert!(
        Config::load(&p.json(
            "erislint.json",
            &json!({"version":1,"rule_files":["rules.json"]})
        ))
        .is_err()
    );
    let c = p.config(json!({"version":2,"extends":["base.json"]}));
    let f = p.write("a.py", "def a(): pass\n");
    assert_eq!(Plan::build(&c, &[f]).unwrap().evaluations.len(), 1);
}

#[test]
fn global_scope_precedes_python_selection_but_selected_extensions_fail() {
    let p = Project::new();
    p.write("src/good.py", "def good(): pass\n");
    let other = p.write("src/ignored.cpp", "not Python {");
    let notes = p.write("src/notes.txt", "bad syntax {");
    let mut value = config(vec![python_rule("r", "function")]);
    value["python_files"] = json!(["src/**"]);
    value["include"] = json!(["src/**/*.py"]);
    value["exclude"] = json!(["src/ignored.cpp"]);
    let c = p.config(value.clone());
    assert_eq!(Plan::build(&c, &[]).unwrap().evaluations.len(), 1);
    for path in [&other, &notes] {
        assert!(
            Plan::from_source(&c, path, "invalid {")
                .unwrap()
                .evaluations
                .is_empty()
        );
    }
    value["include"] = json!(["src/**"]);
    let c = p.config(value.clone());
    assert!(error(Plan::build(&c, &[])).contains("unsupported extension"));
    assert!(error(Plan::from_source(&c, &notes, "invalid")).contains("unsupported extension"));
    assert!(
        error(Plan::from_source(&c, &p.root().join("missing.py"), ""))
            .contains("cannot open input")
    );
}

#[test]
fn overlapping_language_globs_are_rejected_in_disk_and_editor_paths() {
    let p = Project::new();
    let path = p.write("sample.py", "def f(): pass\n");
    let mut c_rule = rule("c");
    c_rule["where"]["language"] = json!("c");
    let mut value = config(vec![python_rule("p", "function"), c_rule]);
    value["c_files"] = json!(["**"]);
    value["include"] = json!(["**/*.py"]);
    let c = p.config(value);
    assert!(error(Plan::build(&c, &[])).contains("both c_files and python_files"));
    assert!(
        error(Plan::from_source(&c, &path, "def f(): pass"))
            .contains("both c_files and python_files")
    );
}

#[test]
fn mixed_language_caches_keep_adapters_across_orders_and_directories() {
    let p = Project::new();
    p.write(
        "Cargo.toml",
        "[package]\nname='test'\nversion='0.1.0'\nedition='2024'\n",
    );
    let mut paths = Vec::new();
    for dir in ["one", "two"] {
        paths.push(p.write(&format!("{dir}/a.rs"), "fn rust_fn() {}"));
        paths.push(p.write(&format!("{dir}/b.c"), "int c_fn(void) { return 1; }"));
        paths.push(p.write(&format!("{dir}/c.py"), "async def python_fn(): return 1\n"));
    }
    let mut c_rule = rule("c");
    c_rule["where"]["language"] = json!("c");
    let mut value = config(vec![
        rule("rust"),
        c_rule,
        python_rule("python", "function"),
    ]);
    value["c_files"] = json!(["**/*.c"]);
    let c = p.config(value);
    let expected = serde_json::to_value(Plan::build(&c, &[]).unwrap()).unwrap();
    assert_eq!(expected["evaluations"].as_array().unwrap().len(), 6);
    paths.reverse();
    paths.push(paths[0].clone());
    assert_eq!(
        serde_json::to_value(Plan::build(&c, &paths).unwrap()).unwrap(),
        expected
    );
    for e in expected["evaluations"].as_array().unwrap() {
        let lang = e["request"]["state"]["language"].as_str().unwrap();
        let id = if lang == "rust" {
            "rust"
        } else if lang == "c" {
            "c"
        } else {
            "python"
        };
        assert!(e["request"]["questions"].get(id).is_some());
    }
}

#[test]
fn explicit_filters_and_overrides_skip_python_but_active_missing_targets_fail() {
    let p = Project::new();
    let path = p.write("sample.py", "not valid {");
    let mut r = python_rule("p", "function");
    r["where"]["exclude"] = json!(["sample.py"]);
    let c = p.config(config(vec![r]));
    assert!(Plan::build(&c, &[]).unwrap().evaluations.is_empty());
    let mut value = config(vec![python_rule("p", "function")]);
    value["overrides"] = json!([{"files":["**"],"rules":{"p":"off"}}]);
    let c = p.config(value);
    assert!(
        Plan::from_source(&c, &path, "invalid {")
            .unwrap()
            .evaluations
            .is_empty()
    );
    let c = p.config(config(vec![python_rule("p", "function")]));
    assert!(error(Plan::from_source(&c, &path, "x=1\n")).contains("no supported Python targets"));
}

#[test]
fn python_editor_snapshots_diagnostics_and_target_selection_share_pipeline() {
    let p = Project::new();
    let path = p.write("source.py", "saved = 1\n");
    p.write("Cargo.toml", "deliberately invalid Cargo manifest");
    let c = p.config(config(vec![python_rule("p", "function")]));
    let source = "# 🦀\nclass C:\n    async def café(self): return 1\n";
    let mut plan = Plan::from_source(&c, &path, source).unwrap();
    assert_eq!(plan.evaluations.len(), 1);
    let start = source.find("café").unwrap();
    plan.select_function(start).unwrap();
    let report: erislint::jev::Response = serde_json::from_value(response("p", 0.8)).unwrap();
    let ds = diagnostics(&c, &plan.evaluations[0], report).unwrap();
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].location.span.start, start);
    assert_eq!(std::fs::read_to_string(path).unwrap(), "saved = 1\n");
    assert_eq!(plan.source(std::path::Path::new("source.py")), Some(source));
}
