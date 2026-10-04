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

#[test]
fn mos_cpp_records_precede_literals_structure_and_markers() {
    let cpp = Options {
        preprocessing: Preprocessing::CppUnexpanded,
        ..options()
    };
    let source = "#define BODY \\\n .macro hidden \\\n .intel_syntax noprefix \\\n #APP \\\n ; erislint-region-begin fake\n";
    for input in [source.to_owned(), source.replace('\n', "\r\n")] {
        let parsed = tokenize(&input, cpp).unwrap();
        assert_eq!(parsed.records.len(), 1);
        assert_eq!(parsed.records[0].kind, "cpp");
        assert_eq!(parsed.records[0].source, input);
        assert!(parsed.regions.is_empty());
        assert!(parsed.statements.is_empty());
        assert!(error(tokenize(&input, options())).contains("preprocessing_mode_required"));
    }
    for head in [
        "define X",
        "undef X",
        "include \"missing.inc\"",
        "line 99 \"missing.s\"",
        "99 \"missing.s\"",
        "error text",
        "warning text",
        "pragma text",
    ] {
        let input = format!("  #{head}\nlda #1\n");
        let parsed = tokenize(&input, cpp).unwrap();
        assert!(parsed.records.iter().any(|r| r.kind == "cpp"));
        assert!(error(tokenize(&input, options())).contains("preprocessing_mode_required"));
    }
    for source in [
        "#define X \\",
        "#define X \\\n",
        "#define X \\\r\n",
        "#define X 1\n.cpu other\n",
        "#if 0\n#else\n#else\n#endif\n",
        "#endif\n",
    ] {
        assert!(tokenize(source, cpp).is_err(), "{source:?}");
    }
    for source in [
        "#define X \"unclosed\nlda #1\n",
        "/* #define X \\\n.cpu other */\n",
        ".ascii \"#define X ; erislint-region-begin fake\"\n",
    ] {
        tokenize(source, cpp).unwrap();
    }
    let parsed = tokenize(
        "#if 0\nlda #1\n#elif FLAG\nlda #2\n#else\nlda #3\n#endif\n",
        cpp,
    )
    .unwrap();
    assert!(
        parsed
            .dependencies
            .iter()
            .any(|r| r.kind == "conditional_body")
    );
}

#[test]
fn mos_structural_context_is_balanced_and_forbidden_state_never_executes() {
    let source = ".macro outer p\n.irp n,a,b\n.irpc c,ab\n.rept 2\n.byte \\p\n; erislint-region-begin fake\n.endr\n.endr\n.endr\n.endm\n.if 0\nunknown_call\n.elseif FLAG\nother\n.else\nopaque\n.endif\n";
    let parsed = tokenize(source, options()).unwrap();
    assert!(parsed.regions.is_empty());
    assert_eq!(
        parsed
            .dependencies
            .iter()
            .filter(|r| r.kind == "macro_body")
            .count(),
        1
    );
    assert_eq!(
        parsed
            .dependencies
            .iter()
            .filter(|r| r.kind == "repetition_body")
            .count(),
        3
    );
    assert_eq!(
        parsed
            .dependencies
            .iter()
            .filter(|r| r.kind == "conditional_body")
            .count(),
        1
    );
    for source in [
        ".endm",
        ".endr",
        ".macro",
        ".macro m\n",
        ".rept\n.endr",
        ".irp p,a\n",
        ".irpc p,ab\n",
        ".rept 2\n.endm",
        ".macro a\n.macro b\n.endm\n.endm",
        ".ifc a,b\n.endif",
        ".else",
        ".if 0\n.else\n.else\n.endif",
        ".if 0\n.endr",
    ] {
        assert!(tokenize(source, options()).is_err(), "{source}");
    }
    for directive in [
        ".code16",
        ".code32",
        ".code64",
        ".intel_syntax",
        ".att_syntax",
        ".altmacro",
        ".noaltmacro",
        ".cpu",
        ".setcpu",
        ".arch",
        ".machine",
        ".syntax",
    ] {
        for source in [
            format!("{directive}\n"),
            format!(".macro m\n{directive}\n.endm\n"),
            format!(".if 0\n{directive}\n.endif\n"),
            format!(".rept 0\n{directive}\n.endr\n"),
        ] {
            assert!(error(tokenize(&source, options())).contains("unsupported assembly state"));
        }
    }
    for preprocessing in [Preprocessing::None, Preprocessing::CppUnexpanded] {
        let opts = Options {
            preprocessing,
            ..options()
        };
        for source in [
            "#APP",
            " \t#NO_APP \t\r\n",
            ".macro m\n#APP\n.endm\n",
            ".if 0\n#NO_APP\n.endif\n",
        ] {
            assert!(error(tokenize(source, opts)).contains("scrub control"));
        }
        for source in [
            "#APP extra\n",
            "; #NO_APP\n",
            "/* #APP */\n",
            ".ascii \".cpu #APP\"\n",
        ] {
            tokenize(source, opts).unwrap();
        }
    }
}

#[test]
fn mos_regions_retain_exact_unicode_crlf_ranges_in_every_context() {
    let source = "  ; erislint-region-begin r\r\n\t.byte \"é\"\r\n  ; erislint-region-end r\r\n";
    assert_eq!(source.len(), 69);
    let parsed = tokenize(source, options()).unwrap();
    let r = &parsed.regions[0];
    assert_eq!((r.body.span.start, r.body.span.end), (29, 42));
    assert_eq!((r.name_span.start, r.name_span.end), (26, 27));
    assert_eq!((r.begin_line.span.start, r.begin_line.span.end), (0, 29));
    assert_eq!((r.end_line.span.start, r.end_line.span.end), (42, 69));
    assert_eq!(
        (r.begin_comment.span.start, r.begin_comment.span.end),
        (2, 27)
    );
    assert_eq!((r.end_comment.span.start, r.end_comment.span.end), (44, 67));
    assert_eq!((r.name_span.line, r.name_span.column), (1, 27));
    let text = assembly::text::Text::new(source);
    assert_eq!(&source[37..39], "é");
    let span = text.span(37, 39);
    assert_eq!((span.line, span.column, span.end_column), (2, 9, 10));
    let targets = assembly::extract(
        source,
        options(),
        &BTreeSet::from([TargetKind::File, TargetKind::AssemblyRegion]),
    )
    .unwrap();
    for context in [
        InputContext::Target,
        InputContext::Enclosing,
        InputContext::File,
    ] {
        assert_eq!(targets[1].input(context, source)["source"], &source[29..42]);
        assert_eq!(targets[0].input(context, source)["source"], source);
    }
    let eof = source.trim_end_matches("\r\n");
    assert_eq!(
        tokenize(eof, options()).unwrap().regions[0]
            .end_line
            .span
            .end,
        67
    );
    for source in ["", " \t\r\n", "; é no final newline"] {
        let ts = assembly::extract(source, options(), &BTreeSet::from([TargetKind::File])).unwrap();
        assert_eq!(ts[0].range.end, source.len());
        assert_eq!(ts[0].input(InputContext::Target, source)["source"], source);
    }
    let decorated = "; é preamble\n\t; erislint-region-begin loop.1-a \t\n; body\n\t; erislint-region-end loop.1-a\n; trailer\n";
    assert_eq!(
        tokenize(decorated, options()).unwrap().regions[0]
            .body
            .source,
        "; body\n"
    );
}

#[test]
fn mos_regions_reject_malformed_pairs_and_ignore_inert_markers() {
    for name in [
        "",
        "1bad",
        "é",
        "bad/name",
        "bad:name",
        "two names",
        &"a".repeat(65),
    ] {
        let source = format!("; erislint-region-begin {name}\nnop\n; erislint-region-end {name}\n");
        assert!(tokenize(&source, options()).is_err(), "{name}");
    }
    for name in ["r", "_start", "loop.1-a", &"a".repeat(64)] {
        let source =
            format!("; erislint-region-begin {name}\n; body\n; erislint-region-end {name}");
        assert_eq!(tokenize(&source, options()).unwrap().regions[0].name, name);
        assert!(tokenize(&format!("{source}\n{source}"), options()).is_err());
    }
    for body in ["", " \t\r\n", "\u{2003}\n"] {
        assert!(
            tokenize(
                &format!("; erislint-region-begin r\n{body}; erislint-region-end r\n"),
                options()
            )
            .is_err()
        );
    }
    for source in [
        "; erislint-region-begin r",
        "; erislint-region-end r\n",
        "; erislint-region-begin r\nnop\n",
        "; erislint-region-begin r\n; erislint-region-begin x\n",
        "; erislint-region-begin r\nnop\n; erislint-region-end x\n",
        "; erislint-region-begin r trailing prose\n",
    ] {
        assert!(tokenize(source, options()).is_err(), "{source}");
    }
    let source = "/*\n; erislint-region-begin fake\n*/\n// ; erislint-region-begin fake\n# ; erislint-region-end fake\n.ascii \"; erislint-region-begin fake\"\n.macro m\n; erislint-region-begin fake\n.endm\n.rept 0\n; erislint-region-end fake\n.endr\nnop ; erislint-region-begin inline\n;erislint-region-begin no_space\n";
    assert!(tokenize(source, options()).unwrap().regions.is_empty());
}

#[test]
fn mos_request_snapshots_cover_both_targets_and_essential_unknowns_in_all_contexts() {
    let p = Project::new();
    p.write(
        "review.S",
        include_str!("../examples/assembly-mos/review.S"),
    );
    p.write(
        "erislint.json",
        include_str!("../examples/assembly-mos/erislint.json"),
    );
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_erislint"))
        .current_dir(p.root())
        .env_remove("jev_key")
        .arg("--dry-run")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        out.stdout,
        include_bytes!("fixtures/assembly-mos-requests.json")
    );
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    let es = value["evaluations"].as_array().unwrap();
    assert_eq!(es.len(), 6);
    let analysis = &es[0]["request"]["state"]["analysis"];
    for e in es {
        assert_eq!(&e["request"]["state"]["analysis"], analysis);
        assert_eq!(e["request"]["questions"].as_object().unwrap().len(), 1);
        for kind in ["cpp", "directive", "macro_body", "conditional_body"] {
            assert!(
                analysis["dependencies"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r["kind"] == kind),
                "{kind}"
            );
        }
        assert_eq!(analysis["validated_cpu_features"], json!([]));
    }
}

#[test]
fn mixed_profiles_languages_options_and_embedded_rust_asm_keep_separate_requests() {
    let p = Project::new();
    p.write(
        "Cargo.toml",
        "[package]\nname='mixed'\nversion='0.1.0'\nedition='2024'\n",
    );
    let paths = [
        p.write(
            "a.rs",
            "fn a(){ core::arch::asm!(\"lda #1\"); } core::arch::global_asm!(\"lda #2\");",
        ),
        p.write("b.c", "int b(void){return 0;}"),
        p.write("c.py", "def c(): pass"),
        p.write("x.s", ".byte 'A\nvalue = 4/2\n"),
        p.write("x.S", "#define X 1\nnop /note\n"),
        p.write("mos.s", ".byte 'A'\nlda #1;note\n"),
        p.write("mos.S", "#define X 1\nlda #X\n"),
        p.write("nested/mos.s", "lda/*x*/#1\n"),
    ];
    let mut c = rule("c");
    c["where"]["language"] = json!("c");
    let mut py = rule("p");
    py["where"]["language"] = json!("python");
    let mut x = mos_rule("x", "file");
    x["where"]["profile"] = json!("x86-gas-att32");
    let mut v = config(vec![rule("r"), c, py, x, mos_rule("m", "file")]);
    v["include"] = json!(["**"]);
    v["c_files"] = json!(["**/*.c"]);
    v["python_files"] = json!(["**/*.py"]);
    v["assembly_sources"] = json!([
        {"files":["x.s"],"profile":"x86-gas-att32","preprocessing":"none","slash_mode":"divide"},
        {"files":["x.S"],"profile":"x86-gas-att32","preprocessing":"cpp-unexpanded","slash_mode":"gas-default"},
        {"files":["mos.s","nested/*.s"],"profile":"mos-llvm-c64","preprocessing":"none"},
        {"files":["mos.S"],"profile":"mos-llvm-c64","preprocessing":"cpp-unexpanded"}]);
    let c = p.config(v.clone());
    let expected = serde_json::to_value(Plan::build(&c, &[]).unwrap()).unwrap();
    assert_eq!(expected["evaluations"].as_array().unwrap().len(), 8);
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
    for (i, path) in paths.iter().enumerate() {
        let source = std::fs::read_to_string(path).unwrap();
        let plan = Plan::from_source(&c, path, &source).unwrap();
        assert_eq!(plan.evaluations.len(), 1);
        let e = &plan.evaluations[0];
        let id = if i == 0 {
            "r"
        } else if i == 1 {
            "c"
        } else if i == 2 {
            "p"
        } else if i < 5 {
            "x"
        } else {
            "m"
        };
        assert!(e.request.questions.contains_key(id));
        if i >= 3 {
            assert_eq!(e.kind, TargetKind::File);
        }
        if i == 0 {
            assert_eq!(e.kind, TargetKind::Function);
            assert!(e.request.state.get("analysis").is_none());
        }
    }
    v["assembly_sources"][2]["files"] = json!(["**/*.s"]);
    assert!(error(Plan::build(&p.config(v), &[])).contains("multiple assembly"));
}

#[test]
fn mos_scope_pairing_and_unsaved_paths_follow_existing_dispatch_contract() {
    let p = Project::new();
    let path = p.write("src/a.s", "lda #1\n");
    let ignored = p.write("src/ignored.txt", ".cpu other");
    let mut v = config(vec![mos_rule("m", "file")]);
    v["include"] = json!(["src/**"]);
    v["exclude"] = json!(["src/ignored.txt"]);
    v["assembly_sources"][0]["files"] = json!(["src/**"]);
    let c = p.config(v.clone());
    assert_eq!(Plan::build(&c, &[]).unwrap().files, 1);
    assert!(
        Plan::from_source(&c, &ignored, ".cpu other")
            .unwrap()
            .evaluations
            .is_empty()
    );
    v["exclude"] = json!([]);
    assert!(error(Plan::build(&p.config(v.clone()), &[])).contains("unsupported extension"));
    v["exclude"] = json!(["src/ignored.txt"]);
    let c = p.config(v.clone());
    assert!(
        error(Plan::from_source(&c, &p.root().join("missing.s"), "nop"))
            .contains("cannot open input")
    );
    let outside = Project::new();
    let other = outside.write("other.s", "nop");
    assert!(Plan::from_source(&c, &other, "nop").is_err());
    let snapshot = "; erislint-region-begin r\nlda #2\n; erislint-region-end r\n";
    assert_eq!(
        Plan::from_source(&c, &path, snapshot).unwrap().evaluations[0]
            .request
            .state["source"],
        snapshot
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "lda #1\n");
    v["rules"][0]["where"]["kind"] = json!("assembly_region");
    let c = p.config(v.clone());
    assert!(error(Plan::build(&c, &[])).contains("no assembly regions"));
    assert_eq!(
        Plan::from_source(&c, &path, snapshot).unwrap().evaluations[0].target,
        "r"
    );
    v["overrides"] = json!([{"files":["**"],"rules":{"m":"off"}}]);
    assert!(
        Plan::from_source(&p.config(v.clone()), &path, ".cpu invalid")
            .unwrap()
            .evaluations
            .is_empty()
    );
    v["rules"][0]["where"]["profile"] = json!("x86-gas-att32");
    assert!(Config::load(&p.json("erislint.json", &v)).is_err());
    std::fs::write(&path, [0xff]).unwrap();
    assert!(Plan::build(&c, &[]).is_err());
}

#[test]
fn mos_inheritance_and_external_rules_keep_declaring_version_validation() {
    let p = Project::new();
    let valid = config(vec![mos_rule("m", "file")]);
    p.json("base.json", &valid);
    let c = p.config(json!({"version":3,"extends":["base.json"]}));
    assert_eq!(c.rules["m"].origin.version, 3);
    assert!(c.rules["m"].origin.path.ends_with("base.json"));
    for version in [1, 2] {
        assert!(
            error(Config::load(&p.json(
                "erislint.json",
                &json!({"version":version,"extends":["base.json"]})
            )))
            .contains("cannot extend version 3")
        );
        p.json("rules.json", &mos_rule("m", "file"));
        let mut v = config(vec![]);
        v["rule_files"] = json!([{"path":"rules.json","version":version}]);
        assert!(Config::load(&p.json("erislint.json", &v)).is_err());
    }
    let mut bad = valid.clone();
    bad["assembly_sources"][0]["slash_mode"] = json!("divide");
    p.json("base.json", &bad);
    let mut child = valid;
    child["extends"] = json!(["base.json"]);
    assert!(error(Config::load(&p.json("erislint.json", &child))).contains("base.json"));
    for criterion in [Value::Null, json!("wrong")] {
        let mut r = mos_rule("m", "file");
        if criterion.is_null() {
            r["question"]["criteria"]
                .as_object_mut()
                .unwrap()
                .remove("insufficient_context");
        } else {
            r["question"]["criteria"]["insufficient_context"] = criterion;
        }
        p.json("rules.json", &r);
        let mut v = config(vec![]);
        v["rule_files"] = json!(["rules.json"]);
        assert!(Config::load(&p.json("erislint.json", &v)).is_err());
    }
}

#[test]
fn mos_uncertainty_is_a_fixed_warning_and_retains_unfiltered_answers() {
    use erislint::{
        policy::Level,
        runner::{diagnostics, rule_answers},
    };
    let p = Project::new();
    p.write("a.s", "lda #unknown\n");
    for setting in ["warn", "error"] {
        let mut v = config(vec![mos_rule("m", "file")]);
        v["overrides"] = json!([{"files":["**"],"rules":{"m":setting}}]);
        let c = p.config(v);
        let plan = Plan::build(&c, &[]).unwrap();
        let e = &plan.evaluations[0];
        for confidence in [0.0, 0.99] {
            let response=serde_json::from_value(json!({"model":"mock","answers":{"m":{"type":"choice","choice":"insufficient_context","confidence":confidence,"probabilities":{"good":0.1,"bad":0.1,"insufficient_context":0.8}}}})).unwrap();
            let answers = rule_answers(e, &response);
            let ds = diagnostics(&c, e, response).unwrap();
            assert_eq!(ds.len(), 1);
            assert_eq!(ds[0].level, Level::Warn);
            assert_eq!(ds[0].message, erislint::config::assembly::INCONCLUSIVE);
            assert_eq!(answers[0].answer.choice, "insufficient_context");
            assert_eq!(answers[0].model, "mock");
            assert_eq!(answers[0].answer.probabilities.len(), 3);
        }
    }
    for condition in [
        json!({}),
        json!({"choice":"insufficient_context"}),
        json!({"choice":"bad","all":[{"probability":{"choice":"insufficient_context","min":0.1}}]}),
    ] {
        let mut r = mos_rule("m", "file");
        r["diagnostics"][0]["when"] = condition;
        assert!(Config::load(&p.json("erislint.json", &config(vec![r]))).is_err());
    }
}

#[test]
fn mos_cli_unsaved_regions_are_offline_and_leave_disk_unchanged() {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let p = Project::new();
    let path = p.write("a.s", "lda #1\n");
    p.config(config(vec![mos_rule("m", "assembly_region")]));
    let source = "; erislint-region-begin changed\nlda #2\n; erislint-region-end changed\n";
    let mut child = Command::new(env!("CARGO_BIN_EXE_erislint"))
        .current_dir(p.root())
        .env_remove("jev_key")
        .args(["--dry-run", "--stdin-file"])
        .arg(&path)
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
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["evaluations"][0]["target"], "changed");
    assert_eq!(
        value["evaluations"][0]["request"]["state"]["source"],
        "lda #2\n"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "lda #1\n");
    let mut v = config(vec![mos_rule("m", "file")]);
    v["overrides"] = json!([{"files":["**"],"rules":{"m":"off"}}]);
    p.config(v);
    let out = Command::new(env!("CARGO_BIN_EXE_erislint"))
        .current_dir(p.root())
        .env_remove("jev_key")
        .args(["--format", "json", "--deny-warnings", "--errors-only"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["evaluations"], 0);
    assert_eq!(value["questions"], 0);
}
