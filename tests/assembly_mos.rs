// Fork-specific LLVM-MOS generic source contracts. Synthetic source only.
// SPDX-License-Identifier: AGPL-3.0-only
mod common;
use common::{Project, rule};
use erislint::{
    assembly::{self, lexer::tokenize},
    config::{
        Config, InputContext,
        assembly::{Options, Preprocessing, Profile, SlashMode, UNCERTAINTY_DESCRIPTION},
    },
    runner::Plan,
    source::TargetKind,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

fn options() -> Options {
    Options {
        profile: Profile::MosLlvmC64,
        preprocessing: Preprocessing::None,
        slash_mode: None,
    }
}
fn mos_rule(id: &str, kind: &str) -> Value {
    let mut r = rule(id);
    r["where"] = json!({"language":"assembly","profile":"mos-llvm-c64","kind":kind});
    r["question"]["criteria"]
        .as_object_mut()
        .unwrap()
        .remove("unknown");
    r["question"]["criteria"]["insufficient_context"] = json!(UNCERTAINTY_DESCRIPTION);
    r
}
fn config(rules: Vec<Value>) -> Value {
    json!({"version":3,"include":["**/*.s","**/*.S"],"assembly_sources":[{"files":["**/*.s","**/*.S"],"profile":"mos-llvm-c64","preprocessing":"none"}],"rules":rules})
}
fn error<T>(result: anyhow::Result<T>) -> String {
    match result {
        Ok(_) => panic!("expected error"),
        Err(e) => format!("{e:#}"),
    }
}

#[test]
fn mos_profile_requires_explicit_intent_and_rejects_gas_options() {
    let p = Project::new();
    let valid = config(vec![mos_rule("m", "file")]);
    p.write("review.s", "lda #$01 ; source only\n");
    assert_eq!(Plan::build(&p.config(valid.clone()), &[]).unwrap().files, 1);
    assert_eq!(
        p.config(valid.clone()).assembly_sources[0].options,
        options()
    );
    for mode in ["gas-default", "divide"] {
        let mut v = valid.clone();
        v["assembly_sources"][0]["slash_mode"] = json!(mode);
        assert!(
            error(Config::load(&p.json("erislint.json", &v)))
                .contains("does not accept GAS slash_mode")
        );
    }
    for field in ["profile", "preprocessing", "files"] {
        let mut v = valid.clone();
        v["assembly_sources"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(Config::load(&p.json("erislint.json", &v)).is_err());
    }
    for profile in ["ca65", "xa65", "mos6502x", "mos6510", "mos-llvm-65c02"] {
        let mut v = valid.clone();
        v["assembly_sources"][0]["profile"] = json!(profile);
        assert!(Config::load(&p.json("erislint.json", &v)).is_err());
    }
    let invalid = Options {
        slash_mode: Some(SlashMode::Divide),
        ..options()
    };
    assert!(tokenize("", invalid).is_err());
}

#[test]
fn mos_comments_hash_state_and_statement_boundaries_are_profile_specific() {
    let cases: &[(&str, &[(&str, &str)])] = &[
        (
            "lda #1;note",
            &[
                ("atom", "lda"),
                ("whitespace", " "),
                ("punctuation", "#"),
                ("atom", "1"),
                ("comment", ";note"),
            ],
        ),
        (
            "lda #1//note",
            &[
                ("atom", "lda"),
                ("whitespace", " "),
                ("punctuation", "#"),
                ("atom", "1"),
                ("comment", "//note"),
            ],
        ),
        (
            "lda/*note*/#1",
            &[
                ("atom", "lda"),
                ("comment", "/*note*/"),
                ("punctuation", "#"),
                ("atom", "1"),
            ],
        ),
        ("  # note", &[("whitespace", "  "), ("comment", "# note")]),
        (
            "label: #1",
            &[
                ("atom", "label"),
                ("punctuation", ":"),
                ("whitespace", " "),
                ("punctuation", "#"),
                ("atom", "1"),
            ],
        ),
        (
            "/*x*/#1",
            &[("comment", "/*x*/"), ("punctuation", "#"), ("atom", "1")],
        ),
        (
            "/*x\ny*/#1\n",
            &[
                ("comment", "/*x\ny*/"),
                ("punctuation", "#"),
                ("atom", "1"),
                ("newline", "\n"),
            ],
        ),
        ("4/2", &[("atom", "4"), ("punctuation", "/"), ("atom", "2")]),
    ];
    for (source, expected) in cases {
        let parsed = tokenize(source, options()).unwrap();
        assert_eq!(
            parsed
                .records
                .iter()
                .map(|r| (r.kind, r.source))
                .collect::<Vec<_>>(),
            *expected,
            "{source}"
        );
        let mut offset = 0;
        for r in parsed.records {
            assert_eq!(r.span.start, offset);
            offset += r.source.len();
            assert_eq!(r.span.end, offset);
        }
        assert_eq!(offset, source.len());
    }
    let source = "nop; nop # note\n";
    assert_eq!(tokenize(source, options()).unwrap().statements.len(), 1);
    let x86 = Options {
        profile: Profile::X86GasAtt32,
        slash_mode: Some(SlashMode::GasDefault),
        ..options()
    };
    assert_eq!(tokenize(source, x86).unwrap().statements.len(), 2);
    let parsed = tokenize("/*x\ny*/ \t#1\n  # note\n", options()).unwrap();
    assert_eq!(
        parsed
            .records
            .iter()
            .filter(|r| r.kind == "comment")
            .map(|r| r.source)
            .collect::<Vec<_>>(),
        ["/*x\ny*/", "# note"]
    );
}

#[test]
fn mos_paired_characters_and_bounded_strings_preserve_delimiters() {
    let source = ".byte 'A', '\\n', '\\'', '#', ';';real\n.ascii \"é ;#/*x*/\\\"\\\\\\b\\f\\n\\r\\t\" //real\n";
    let parsed = tokenize(source, options()).unwrap();
    assert_eq!(
        parsed.records.iter().map(|r| r.source).collect::<String>(),
        source
    );
    assert_eq!(
        parsed
            .records
            .iter()
            .filter(|r| r.kind == "character")
            .map(|r| r.source)
            .collect::<Vec<_>>(),
        ["'A'", "'\\n'", "'\\''", "'#'", "';'"]
    );
    assert_eq!(
        parsed
            .records
            .iter()
            .filter(|r| r.kind == "comment")
            .map(|r| r.source)
            .collect::<Vec<_>>(),
        [";real", "//real"]
    );
    for source in [
        "'A",
        "''",
        "'''",
        "'AB'",
        "'é'",
        "'\\q'",
        "'\n'",
        "'\r\n'",
        "'",
        "\"x\n\"",
        "\"x\r\n\"",
        "\"x\\\n\"",
        "\"\\q\"",
        "\"\\'\"",
        "\"unclosed",
    ] {
        assert!(tokenize(source, options()).is_err(), "{source:?}");
    }
    for source in [
        "name/*x*/suffix",
        "/*x*/'A';note",
        "/*a\r\nb*/\r\n",
        "lda #1\\\r\n +2\n",
    ] {
        let parsed = tokenize(source, options()).unwrap();
        assert_eq!(
            parsed.records.iter().map(|r| r.source).collect::<String>(),
            source
        );
    }
    for source in [
        "/*unclosed",
        "/*a/*b*/c*/",
        "*/",
        "/*a\rb*/",
        "/*a*/\r",
        "lda #1\\\r",
    ] {
        assert!(tokenize(source, options()).is_err(), "{source:?}");
    }
}

#[test]
fn mos_operands_labels_and_cpu_intent_remain_unvalidated_source() {
    let source = ".section .text\n.set ALIAS, unknown\nfirst: lda #$01\n.Lnext: sta $d000\n1: lda ($20),y\nbne 1b\nbne 1f\n1: opaque_op symbol@mos16lo\n.byte %1010\n.word ALIAS\n";
    let parsed = tokenize(source, options()).unwrap();
    assert_eq!(
        parsed
            .statements
            .iter()
            .filter(|r| r.kind == "label")
            .map(|r| r.source)
            .collect::<Vec<_>>(),
        ["first:", ".Lnext:", "1:", "1:"]
    );
    assert!(parsed.records.iter().any(|r| r.source == "1b"));
    assert!(parsed.records.iter().any(|r| r.source == "1f"));
    let targets =
        assembly::extract(source, options(), &BTreeSet::from([TargetKind::File])).unwrap();
    assert_eq!(targets.len(), 1);
    let input = targets[0].input(InputContext::Target, source);
    assert_eq!(input["source"], source);
    assert_eq!(input["analysis"]["validated_cpu_features"], json!([]));
    assert_eq!(input["analysis"]["completeness"], "incomplete");
    assert!(
        input["analysis"]["cpu_intent"]
            .as_str()
            .unwrap()
            .contains("not a validated LLVM CPU")
    );
    assert!(input["analysis"]["options"].get("slash_mode").is_none());
    assert!(assembly::extract(source, options(), &BTreeSet::from([TargetKind::Function])).is_err());
}
