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
