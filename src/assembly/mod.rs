// Fork-specific bounded assembly source review.
// SPDX-License-Identifier: AGPL-3.0-only
pub mod text;

pub mod lexer;

use crate::{
    config::assembly::{Options, Profile},
    source::{Target, TargetKind},
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use text::{Record, Text};

/// Lossless source review only. No assembler, preprocessor or analyzed code runs.
pub fn extract(
    source: &str,
    options: Options,
    kinds: &BTreeSet<TargetKind>,
) -> Result<Vec<Target>> {
    ensure!(
        kinds
            .iter()
            .all(|k| matches!(k, TargetKind::File | TargetKind::AssemblyRegion)),
        "assembly supports only file and assembly_region targets"
    );
    let parsed = lexer::tokenize(source, options)?;
    ensure!(
        !kinds.contains(&TargetKind::AssemblyRegion) || !parsed.regions.is_empty(),
        "no assembly regions matched active rules"
    );
    let text = Text::new(source);
    let mut targets = Vec::new();
    if kinds.contains(&TargetKind::File) {
        let body = text.record("file", 0, source.len());
        targets.push(Target::new(
            TargetKind::File,
            "<file>".into(),
            body.span.clone(),
            body.span.clone(),
            false,
            state(options, &parsed, &body),
            vec![],
        ));
    }
    if kinds.contains(&TargetKind::AssemblyRegion) {
        for region in &parsed.regions {
            let mut input = state(options, &parsed, &region.body);
            input["name"] = json!(region.name);
            input["kind"] = json!("assembly_region");
            input["markers"] = json!({"begin_line":region.begin_line,"end_line":region.end_line,"begin_comment":region.begin_comment,"end_comment":region.end_comment});
            targets.push(Target::new(
                TargetKind::AssemblyRegion,
                region.name.clone(),
                region.name_span.clone(),
                region.body.span.clone(),
                false,
                input,
                vec![],
            ));
        }
    }
    ensure!(!targets.is_empty(), "no supported assembly targets found");
    Ok(targets)
}
fn state(options: Options, parsed: &lexer::Parsed<'_>, body: &Record<'_>) -> Value {
    let contained =
        |r: &&Record<'_>| r.span.start >= body.span.start && r.span.end <= body.span.end;
    let records: Vec<_> = parsed.records.iter().filter(contained).collect();
    let statements: Vec<_> = parsed.statements.iter().filter(contained).collect();
    let issues:Vec<_>=parsed.statements.iter().filter(|r|r.kind=="opaque_statement"||r.kind=="directive").map(|r|json!({"reason":if r.kind=="directive" {"directive_not_evaluated"} else {"statement_or_macro_not_resolved"},"span":r.span})).collect();
    let cpu_intent = match options.profile {
        Profile::X86GasAtt32 => "i386-compatible 32-bit; not validated",
        Profile::MosLlvmC64 => "C64/6510 intent; not a validated LLVM CPU or feature selection",
    };
    let mut state = json!({"language":"assembly","kind":"file","name":null,"source":body.source,"records":records,"statements":statements,
        "analysis":{"completeness":"incomplete","parser":"erislint-bounded-assembly-v1","profile":options.profile,"cpu_intent":cpu_intent,"options":options,
            "preprocessing":"raw source only; never executed","dependency_scope":"conservative file-wide; relevance unresolved","dependencies":parsed.dependencies,"issues":issues,
            "unknown":["macro_expansion_unknown","conditional_activity_unknown","assembler_state_unknown","symbols_and_relocations_unresolved","instruction_legality_and_cpu_semantics_not_validated","layout_timing_stack_and_abi_unknown"]}});
    if options.profile == Profile::MosLlvmC64 {
        state["analysis"]["dialect"] = json!("LLVM-MOS generic bounded source subset");
        state["analysis"]["validated_cpu_features"] = json!([]);
        state["analysis"]["reference_revision"] = json!("06bc967d2668c7c11c4d6eb43a6aed1f99ad258b");
    }
    state
}
