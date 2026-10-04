// Copyright (C) 2026 Eriskii
// SPDX-License-Identifier: AGPL-3.0-only
// See LICENSE for the full license text.

// Shared carriers extracted for the fork; Rust extraction remains in rust.rs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::InputContext;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    File,
    Class,
}

/// Byte offsets are zero-based, end-exclusive. Lines and Unicode columns are one-based.
#[derive(Debug, Clone, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

pub struct Target {
    pub kind: TargetKind,
    pub name: String,
    pub span: Span,
    pub range: Span,
    pub has_body: bool,
    state: Value,
    enclosing: Vec<Value>,
}

impl Target {
    pub(crate) fn new(
        kind: TargetKind,
        name: String,
        span: Span,
        range: Span,
        has_body: bool,
        state: Value,
        enclosing: Vec<Value>,
    ) -> Self {
        Self {
            kind,
            name,
            span,
            range,
            has_body,
            state,
            enclosing,
        }
    }

    pub fn input(&self, context: InputContext, source: &str) -> Value {
        let mut state = self.state.clone();
        match context {
            InputContext::Target => {}
            InputContext::Enclosing => state["context"] = json!({ "enclosing": self.enclosing }),
            InputContext::File => {
                state["context"] = json!({ "enclosing": self.enclosing, "file": source })
            }
        }
        state
    }
}
