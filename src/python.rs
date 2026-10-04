// Fork-specific Python source adapter.
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::BTreeSet;

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use tree_sitter::{Node, Parser};

use crate::source::{Span, Target, TargetKind};

/// Inspect UTF-8 source with the pinned grammar; never import or execute it.
pub fn extract(source: &str, kinds: &BTreeSet<TargetKind>) -> Result<Vec<Target>> {
    ensure!(
        kinds.iter().all(|kind| matches!(
            kind,
            TargetKind::Function | TargetKind::Class | TargetKind::File
        )),
        "Python supports only function, class and file targets"
    );
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .context("cannot initialize Python grammar")?;
    let tree = parser
        .parse(source, None)
        .context("Python parsing did not complete")?;
    let root = tree.root_node();
    let positions = Positions::new(source);
    let nodes = descendants(root);
    for node in &nodes {
        if node.kind() == "block" {
            let mut cursor = node.walk();
            ensure!(
                node.named_children(&mut cursor)
                    .any(|child| child.kind() != "comment"),
                "Python empty suites are unsupported; an explicit statement is required"
            );
        }
        if node.is_error() || node.is_missing() {
            let span = positions.span(*node);
            bail!(
                "Python syntax error or unsupported syntax at {}:{} ({})",
                span.line,
                span.column,
                node.kind()
            );
        }
        ensure!(
            !(matches!(node.kind(), "print_statement" | "exec_statement")
                || node.kind() == "string_start" && source[node.byte_range()].contains('`')),
            "Python 2 syntax is unsupported in Python mode"
        );
    }
    ensure!(
        !root.has_error(),
        "Python syntax error or unsupported syntax"
    );
    let comments: Vec<_> = nodes
        .iter()
        .filter(|n| n.kind() == "comment")
        .map(|n| record(*n, source, &positions))
        .collect();
    let mut targets = Vec::new();
    for node in nodes {
        let kind = match node.kind() {
            "module" => TargetKind::File,
            "function_definition" => TargetKind::Function,
            "class_definition" => TargetKind::Class,
            _ => continue,
        };
        if kinds.contains(&kind) {
            targets.push(target(node, kind, source, &positions, &comments)?);
        }
    }
    ensure!(
        !targets.is_empty(),
        "no supported Python targets found; use a file rule for files without selected definitions"
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

fn record(node: Node<'_>, source: &str, positions: &Positions<'_>) -> Value {
    json!({"kind": node.kind(), "source": &source[node.byte_range()], "span": positions.span(node)})
}

fn decorated(node: Node<'_>) -> Node<'_> {
    node.parent()
        .filter(|parent| parent.kind() == "decorated_definition")
        .unwrap_or(node)
}

// Classify a standalone literal without decoding it. Record the original outer
// expression separately from the unwrapped node used for classification.
fn docstring(node: Node<'_>, source: &str, positions: &Positions<'_>) -> Option<Value> {
    let body = node.child_by_field_name("body").unwrap_or(node);
    let mut cursor = body.walk();
    let first = body
        .named_children(&mut cursor)
        .find(|n| n.kind() != "comment")?;
    if first.kind() != "expression_statement" {
        return None;
    }
    let expression = single_expression(first)?;
    let mut literal = expression;
    while literal.kind() == "parenthesized_expression" {
        literal = single_expression(literal)?;
    }
    if !matches!(literal.kind(), "string" | "concatenated_string") {
        return None;
    }
    // Bytes and interpolated strings are expressions, not Python docstrings.
    if descendants(literal).iter().any(|n| {
        n.kind() == "string_start"
            && source[n.byte_range()]
                .chars()
                .any(|c| matches!(c, 'b' | 'B' | 'f' | 'F'))
    }) {
        return None;
    }
    Some(record(expression, source, positions))
}

// An unparenthesized tuple is represented by sibling expressions and commas,
// including a single expression followed by a comma. Neither is a docstring.
fn single_expression(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    if node.children(&mut cursor).any(|child| child.kind() == ",") {
        return None;
    }
    let mut cursor = node.walk();
    let mut children = node
        .named_children(&mut cursor)
        .filter(|child| !matches!(child.kind(), "comment" | "line_continuation"));
    let expression = children.next()?;
    children.next().is_none().then_some(expression)
}

fn target(
    node: Node<'_>,
    kind: TargetKind,
    source: &str,
    positions: &Positions<'_>,
    comments: &[Value],
) -> Result<Target> {
    let name = node.child_by_field_name("name");
    ensure!(
        kind == TargetKind::File || name.is_some(),
        "Python definition is missing its name"
    );
    let label = name.map_or("<file>", |name| &source[name.byte_range()]);
    let decorated = decorated(node);
    let range = if kind == TargetKind::File {
        positions.range(0, source.len())
    } else {
        positions.span(decorated)
    };
    let span = name.map_or_else(|| range.clone(), |name| positions.span(name));
    let mut cursor = decorated.walk();
    let decorators: Vec<_> = decorated
        .named_children(&mut cursor)
        .filter(|n| n.kind() == "decorator")
        .map(|n| record(n, source, positions))
        .collect();
    let mut ancestors = Vec::new();
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if matches!(ancestor.kind(), "function_definition" | "class_definition") {
            ancestors.push(ancestor);
        }
        parent = ancestor.parent();
    }
    let method = kind == TargetKind::Function
        && ancestors
            .first()
            .is_some_and(|n| n.kind() == "class_definition");
    ancestors.reverse();
    let enclosing = ancestors
        .iter()
        .map(|n| record(*n, source, positions))
        .collect();
    let field = |field| {
        node.child_by_field_name(field)
            .map(|n| record(n, source, positions))
    };
    let mut cursor = node.walk();
    let is_async = node.children(&mut cursor).any(|n| n.kind() == "async");
    let state = json!({
        "language": "python", "kind": kind, "name": name.map(|_| label),
        "source": &source[range.start..range.end], "comments": comments,
        "docstring": docstring(node, source, positions), "decorators": decorators,
        "async": is_async, "method": method, "parameters": field("parameters"),
        "return_annotation": field("return_type"), "type_parameters": field("type_parameters"),
        "bases": field("superclasses"), "body": field("body"),
        "analysis": { "completeness": "incomplete", "parser": "tree-sitter-python-0.25.0",
            "syntax_contract": "pinned Python 3 grammar subset; not CPython conformance validation",
            "unknown": ["imports_not_loaded", "types_and_symbols_not_resolved", "decorators_and_metaclasses_not_evaluated", "dynamic_attributes_and_runtime_binding_unknown", "control_flow_not_evaluated"] }
    });
    Ok(Target::new(
        kind,
        label.to_owned(),
        span,
        range,
        node.child_by_field_name("body").is_some(),
        state,
        enclosing,
    ))
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
        self.range(node.start_byte(), node.end_byte())
    }
    fn range(&self, start: usize, end: usize) -> Span {
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
