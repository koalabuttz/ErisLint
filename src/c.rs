// Fork-specific C source adapter.
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::BTreeSet;

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use tree_sitter::{Node, Parser};

use crate::source::{Span, Target, TargetKind};

/// Parse source without compiling, preprocessing, or loading included files.
pub fn extract(source: &str, kinds: &BTreeSet<TargetKind>) -> Result<Vec<Target>> {
    ensure!(
        kinds
            .iter()
            .all(|kind| matches!(kind, TargetKind::Function | TargetKind::File)),
        "C supports only function and file targets"
    );
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c::LANGUAGE.into())
        .context("cannot initialize C grammar")?;
    let tree = parser
        .parse(source, None)
        .context("C parsing did not complete")?;
    let root = tree.root_node();
    let nodes = descendants(root);
    let positions = Positions::new(source);
    if let Some(node) = nodes
        .iter()
        .find(|node| node.is_error() || node.is_missing())
    {
        let span = positions.span(*node);
        bail!(
            "C syntax error or unsupported syntax at {}:{} ({})",
            span.line,
            span.column,
            node.kind()
        );
    }
    ensure!(!root.has_error(), "C syntax error or unsupported syntax");
    ensure!(
        !nodes
            .iter()
            .any(|node| node.kind() == "linkage_specification"),
        "C++ linkage is unsupported in C mode"
    );
    let comments: Vec<_> = nodes
        .iter()
        .filter(|node| node.kind() == "comment")
        .map(|node| record(*node, source, &positions))
        .collect();
    let preprocessor: Vec<_> = nodes
        .iter()
        .filter(|node| node.kind().starts_with("preproc_"))
        .map(|node| record(*node, source, &positions))
        .collect();
    let mut targets = Vec::new();
    if kinds.contains(&TargetKind::File) {
        targets.push(target(
            root,
            None,
            source,
            &positions,
            &comments,
            &preprocessor,
        ));
    }
    for node in nodes {
        match node.kind() {
            "function_definition" => {
                let declarator = node
                    .child_by_field_name("declarator")
                    .context("C function is missing its declarator")?;
                let function =
                    function_name(declarator)?.context("unsupported C function declarator")?;
                if kinds.contains(&TargetKind::Function) {
                    targets.push(target(
                        node,
                        Some(function),
                        source,
                        &positions,
                        &comments,
                        &preprocessor,
                    ));
                }
            }
            "declaration" => {
                let mut cursor = node.walk();
                for declarator in node.children_by_field_name("declarator", &mut cursor) {
                    if let Some(function) = function_name(declarator)?
                        && kinds.contains(&TargetKind::Function)
                    {
                        targets.push(target(
                            node,
                            Some(function),
                            source,
                            &positions,
                            &comments,
                            &preprocessor,
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    ensure!(
        !targets.is_empty(),
        "no supported C targets found; use a file rule for files without functions"
    );
    Ok(targets)
}

fn descendants(root: Node<'_>) -> Vec<Node<'_>> {
    let mut result = Vec::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        result.push(node);
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        pending.extend(children.into_iter().rev());
    }
    result
}

#[derive(Clone, Copy)]
struct Function<'tree> {
    name: Node<'tree>,
    declarator: Node<'tree>,
    signature: Node<'tree>,
}

/// The innermost derived declarator decides whether the name denotes a function
/// or a pointer/array variable. Parentheses alone do not change that binding.
fn function_name(mut node: Node<'_>) -> Result<Option<Function<'_>>> {
    let declarator = node;
    let mut binding = None;
    let mut initialized = false;
    loop {
        match node.kind() {
            "identifier" => {
                ensure!(
                    !initialized || binding.is_none(),
                    "initialized C function declarations are unsupported"
                );
                return Ok(binding.map(|signature| Function {
                    name: node,
                    declarator,
                    signature,
                }));
            }
            "function_declarator" => binding = Some(node),
            "pointer_declarator" | "array_declarator" => binding = None,
            "init_declarator" => initialized = true,
            "parenthesized_declarator" | "attributed_declarator" => {}
            other => bail!("unsupported C declarator {other}"),
        }
        node = if let Some(child) = node.child_by_field_name("declarator") {
            child
        } else {
            let mut cursor = node.walk();
            let mut children = node.named_children(&mut cursor).filter(|child| {
                child.kind() == "identifier" || child.kind().ends_with("_declarator")
            });
            let child = children
                .next()
                .context("unsupported C declarator without a name")?;
            ensure!(children.next().is_none(), "ambiguous C declarator");
            child
        };
    }
}

fn record(node: Node<'_>, source: &str, positions: &Positions<'_>) -> Value {
    json!({"kind": node.kind(), "source": &source[node.byte_range()], "span": positions.span(node)})
}

fn target(
    node: Node<'_>,
    function: Option<Function<'_>>,
    source: &str,
    positions: &Positions<'_>,
    comments: &[Value],
    preprocessor: &[Value],
) -> Target {
    let name = function.map(|function| function.name);
    let signature = function.map(|function| function.signature);
    let kind = if name.is_some() {
        TargetKind::Function
    } else {
        TargetKind::File
    };
    let label = name.map_or("<file>", |name| &source[name.byte_range()]);
    let body = node.child_by_field_name("body");
    let text = |node: Option<Node<'_>>| node.map(|node| &source[node.byte_range()]);
    let mut state = json!({
        "language": "c", "kind": kind, "name": name.map(|_| label),
        "source": &source[node.byte_range()],
        "comments": comments, "preprocessor": preprocessor,
        "analysis": {
            "completeness": "incomplete",
            "parser": "tree-sitter-c-0.24.2",
            "unknown": ["types_and_symbols_not_resolved", "macros_not_expanded", "includes_not_loaded", "conditional_branches_not_evaluated"]
        }
    });
    if kind == TargetKind::Function {
        state["declaration_type"] = json!(text(node.child_by_field_name("type")));
        state["declarator"] = json!(text(function.map(|function| function.declarator)));
        state["parameters"] =
            json!(text(signature.and_then(|signature| {
                signature.child_by_field_name("parameters")
            })));
        state["body"] = json!(text(body));
    }
    let mut enclosing = Vec::new();
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if ancestor.kind().starts_with("preproc_") || ancestor.kind() == "function_definition" {
            enclosing.push(record(ancestor, source, positions));
        }
        parent = ancestor.parent();
    }
    enclosing.reverse();
    Target::new(
        kind,
        label.to_owned(),
        positions.span(name.unwrap_or(node)),
        positions.span(node),
        body.is_some(),
        state,
        enclosing,
    )
}

struct Positions<'a> {
    source: &'a str,
    starts: Vec<usize>,
}

impl<'a> Positions<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            starts: std::iter::once(0)
                .chain(source.match_indices('\n').map(|(offset, _)| offset + 1))
                .collect(),
        }
    }

    fn point(&self, offset: usize) -> (usize, usize) {
        let row = self.starts.partition_point(|&start| start <= offset) - 1;
        (
            row + 1,
            self.source[self.starts[row]..offset].chars().count() + 1,
        )
    }

    fn span(&self, node: Node<'_>) -> Span {
        let start = node.start_byte();
        let end = node.end_byte();
        let (line, column) = self.point(start);
        let (end_line, end_column) = self.point(end);
        Span {
            start,
            end,
            line,
            column,
            end_line,
            end_column,
        }
    }
}
