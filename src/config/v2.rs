// Copyright (C) 2026 Eriskii
// SPDX-License-Identifier: AGPL-3.0-only
// Frozen version-2 schema and input shapes for the fork.
use super::{InputContext, Override, RustEdition};
use crate::{jev::Question, policy::DiagnosticPolicy};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;

/// Project configuration. File patterns are relative to the selected config's directory.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    #[serde(default, rename = "$schema")]
    pub schema: Option<String>,
    #[serde(default = "version")]
    pub version: u32,
    /// Base configurations, applied in order before this configuration.
    #[serde(default)]
    pub extends: Vec<PathBuf>,
    pub model: Option<String>,
    /// Omit to discover the edition from each file's Cargo manifest.
    pub edition: Option<RustEdition>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    /// Version 2 only: explicitly select .c and C-mode .h paths.
    pub c_files: Option<Vec<String>>,
    /// Version 2 only: explicitly select UTF-8 .py source paths.
    pub python_files: Option<Vec<String>>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// Each file contains one rule or an array of rules.
    #[serde(default)]
    pub rule_files: Vec<PathBuf>,
    #[serde(default)]
    pub overrides: Vec<Override>,
}

fn version() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    #[serde(default, rename = "$schema")]
    pub schema: Option<String>,
    pub id: String,
    pub r#where: Selector,
    #[serde(default)]
    pub context: InputContext,
    pub question: Question,
    /// Evaluated in order; the first matching entry determines the diagnostic.
    pub diagnostics: Vec<DiagnosticPolicy>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    /// Version 2 only. Omitted language means Rust.
    pub language: Option<Language>,
    pub kind: TargetKind,
    pub has_body: Option<bool>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    Rust,
    C,
    Python,
}

#[derive(Debug, JsonSchema)]
#[serde(untagged)]
pub enum RuleFile {
    Single(Box<Rule>),
    Multiple(Vec<Rule>),
}

impl<'de> Deserialize<'de> for RuleFile {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let rules = if value.is_array() {
            serde_json::from_value(value).map(Self::Multiple)
        } else {
            serde_json::from_value(value).map(Self::Single)
        };
        rules.map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, JsonSchema)]
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
