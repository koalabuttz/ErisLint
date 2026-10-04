// Copyright (C) 2026 Eriskii
// SPDX-License-Identifier: AGPL-3.0-only
// See LICENSE for the full license text.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{jev::Question, policy::DiagnosticPolicy, source::TargetKind};

pub mod legacy;

const CONFIG_NAME: &str = "erislint.json";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, JsonSchema)]
pub enum RustEdition {
    #[serde(rename = "2015")]
    E2015,
    #[serde(rename = "2018")]
    E2018,
    #[serde(rename = "2021")]
    E2021,
    #[serde(rename = "2024")]
    E2024,
}

impl From<RustEdition> for ra_ap_syntax::Edition {
    fn from(edition: RustEdition) -> Self {
        match edition {
            RustEdition::E2015 => Self::Edition2015,
            RustEdition::E2018 => Self::Edition2018,
            RustEdition::E2021 => Self::Edition2021,
            RustEdition::E2024 => Self::Edition2024,
        }
    }
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

impl Selector {
    pub fn language(&self) -> Language {
        self.language.unwrap_or_default()
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum InputContext {
    /// Only the selected AST node's fields.
    Target,
    /// Add enclosing module, function, impl and trait metadata.
    #[default]
    Enclosing,
    /// Add enclosing metadata and the entire source file.
    File,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Override {
    pub files: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    pub rules: BTreeMap<String, RuleSetting>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuleSetting {
    Off,
    Warn,
    Error,
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

impl RuleFile {
    fn into_rules(self) -> Vec<Rule> {
        match self {
            Self::Single(rule) => vec![*rule],
            Self::Multiple(rules) => rules,
        }
    }
}

pub struct FileFilter {
    include: GlobSet,
    exclude: GlobSet,
}

impl FileFilter {
    pub fn new(include: &[String], exclude: &[String]) -> Result<Self> {
        Ok(Self {
            include: globs(include)?,
            exclude: globs(exclude)?,
        })
    }

    pub fn matches(&self, path: &Path) -> bool {
        (self.include.is_empty() || self.include.is_match(path)) && !self.exclude.is_match(path)
    }
}

fn globs(patterns: &[String]) -> Result<GlobSet> {
    let mut set = GlobSetBuilder::new();
    for pattern in patterns {
        ensure!(!pattern.is_empty(), "file patterns must not be empty");
        set.add(
            GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
                .with_context(|| format!("invalid file pattern {pattern:?}"))?,
        );
    }
    Ok(set.build()?)
}

pub struct CompiledRule {
    pub definition: Rule,
    pub filter: FileFilter,
}

struct CompiledOverride {
    filter: FileFilter,
    rules: BTreeMap<String, RuleSetting>,
}

pub struct Config {
    pub path: PathBuf,
    pub root: PathBuf,
    pub model: String,
    pub edition: Option<RustEdition>,
    pub filter: FileFilter,
    pub c_filter: Option<FileFilter>,
    pub python_filter: Option<FileFilter>,
    pub rules: BTreeMap<String, CompiledRule>,
    overrides: Vec<CompiledOverride>,
}

#[derive(Default)]
struct Merged {
    version: u32,
    model: Option<String>,
    edition: Option<RustEdition>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    c_files: Option<Vec<String>>,
    python_files: Option<Vec<String>>,
    rules: BTreeMap<String, Rule>,
    overrides: Vec<Override>,
}

impl Config {
    pub fn discover(start: &Path) -> Result<PathBuf> {
        let start = start
            .canonicalize()
            .context("cannot resolve working directory")?;
        for directory in start.ancestors() {
            let path = directory.join(CONFIG_NAME);
            if path.is_file() {
                return Ok(path);
            }
            if directory.join(".git").exists() {
                break;
            }
        }
        bail!("no {CONFIG_NAME} found; create one or use --config <path>")
    }

    pub fn load(path: &Path) -> Result<Self> {
        let path = path
            .canonicalize()
            .with_context(|| format!("cannot open config {}", path.display()))?;
        let mut merged = Merged::default();
        merge(&path, &mut Vec::new(), &mut merged)?;
        let rules: BTreeMap<_, _> = merged
            .rules
            .into_iter()
            .map(|(id, definition)| {
                let filter =
                    FileFilter::new(&definition.r#where.files, &definition.r#where.exclude)
                        .with_context(|| format!("rule {id:?}"))?;
                Ok((id, CompiledRule { definition, filter }))
            })
            .collect::<Result<_>>()?;
        ensure!(!rules.is_empty(), "configuration contains no rules");
        let overrides = merged
            .overrides
            .into_iter()
            .map(|entry| {
                ensure!(
                    !entry.files.is_empty(),
                    "overrides require at least one file pattern"
                );
                for id in entry.rules.keys() {
                    ensure!(
                        rules.contains_key(id),
                        "override refers to unknown rule {id:?}"
                    );
                }
                Ok(CompiledOverride {
                    filter: FileFilter::new(&entry.files, &entry.exclude)?,
                    rules: entry.rules,
                })
            })
            .collect::<Result<_>>()?;
        let has_c_rules = rules
            .values()
            .any(|rule| rule.definition.r#where.language() == Language::C);
        ensure!(
            has_c_rules == merged.c_files.is_some(),
            "C rules and explicit c_files must be configured together"
        );
        ensure!(
            merged.c_files.is_none() || merged.version == 2,
            "inherited C configuration requires selected config version 2"
        );
        let c_filter = merged
            .c_files
            .map(|files| FileFilter::new(&files, &[]))
            .transpose()?;
        let has_python_rules = rules
            .values()
            .any(|rule| rule.definition.r#where.language() == Language::Python);
        ensure!(
            has_python_rules == merged.python_files.is_some(),
            "Python rules and explicit python_files must be configured together"
        );
        ensure!(
            merged.python_files.is_none() || merged.version == 2,
            "inherited Python configuration requires selected config version 2"
        );
        let python_filter = merged
            .python_files
            .map(|files| FileFilter::new(&files, &[]))
            .transpose()?;
        let include = merged.include.unwrap_or_else(|| vec!["**/*.rs".into()]);
        ensure!(
            !include.is_empty(),
            "include must contain at least one file pattern"
        );
        Ok(Self {
            root: path
                .parent()
                .context("config has no parent directory")?
                .to_path_buf(),
            path,
            model: merged.model.unwrap_or_else(|| "jev-latest".into()),
            edition: merged.edition,
            c_filter,
            python_filter,
            filter: FileFilter::new(&include, &merged.exclude.unwrap_or_default())?,
            rules,
            overrides,
        })
    }

    pub fn has_source_adapters(&self) -> bool {
        self.c_filter.is_some() || self.python_filter.is_some()
    }

    pub fn setting(&self, path: &Path, rule: &str) -> Option<RuleSetting> {
        self.overrides
            .iter()
            .rev()
            .filter(|entry| entry.filter.matches(path))
            .find_map(|entry| entry.rules.get(rule).copied())
    }
}

fn merge(path: &Path, stack: &mut Vec<PathBuf>, merged: &mut Merged) -> Result<()> {
    ensure!(
        stack.len() < 64,
        "configuration inheritance exceeds 64 levels"
    );
    ensure!(
        !stack.iter().any(|ancestor| ancestor == path),
        "configuration inheritance cycle at {}",
        path.display()
    );
    stack.push(path.to_path_buf());
    let document = read_config(path)?;
    ensure!(
        matches!(document.version, 1 | 2),
        "unsupported configuration version {} in {}",
        document.version,
        path.display()
    );
    ensure!(
        document.version == 2 || document.c_files.is_none(),
        "c_files requires configuration version 2"
    );
    ensure!(
        document.version == 2 || document.python_files.is_none(),
        "python_files requires configuration version 2"
    );
    let directory = path.parent().context("config has no parent directory")?;
    for base in document.extends {
        let base = directory.join(base);
        let base = base
            .canonicalize()
            .with_context(|| format!("cannot open extended config {}", base.display()))?;
        merge(&base, stack, merged)?;
    }
    if let Some(model) = document.model {
        ensure!(!model.trim().is_empty(), "model must not be empty");
        merged.model = Some(model);
    }
    if let Some(edition) = document.edition {
        merged.edition = Some(edition);
    }
    if let Some(include) = document.include {
        merged.include = Some(include);
    }
    if let Some(exclude) = document.exclude {
        merged.exclude = Some(exclude);
    }
    if let Some(files) = document.c_files {
        ensure!(
            !files.is_empty(),
            "c_files must contain at least one file pattern"
        );
        merged.c_files = Some(files);
    }
    if let Some(files) = document.python_files {
        ensure!(
            !files.is_empty(),
            "python_files must contain at least one file pattern"
        );
        merged.python_files = Some(files);
    }
    let mut rules = document.rules;
    for rule_file in document.rule_files {
        rules.extend(read_rule_file(&directory.join(rule_file), document.version)?.into_rules());
    }
    let mut ids = BTreeSet::new();
    for rule in rules {
        ensure!(
            document.version == 2 || rule.r#where.language.is_none(),
            "rule language requires configuration version 2"
        );
        ensure!(
            ids.insert(rule.id.clone()),
            "duplicate rule {:?} in {}",
            rule.id,
            path.display()
        );
        validate_rule(&rule)
            .with_context(|| format!("invalid rule {:?} in {}", rule.id, path.display()))?;
        merged.rules.insert(rule.id.clone(), rule);
    }
    merged.overrides.extend(document.overrides);
    merged.version = document.version;
    stack.pop();
    Ok(())
}

fn validate_rule(rule: &Rule) -> Result<()> {
    ensure!(
        !rule.id.is_empty()
            && rule
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')),
        "rule ids must contain only letters, digits, '.', '_' or '-'"
    );
    ensure!(
        rule.r#where.has_body.is_none() || rule.r#where.kind == TargetKind::Function,
        "has_body is only valid for function targets"
    );
    ensure!(
        rule.r#where.language() != Language::C
            || matches!(rule.r#where.kind, TargetKind::Function | TargetKind::File),
        "C supports only function and file targets"
    );
    ensure!(
        rule.r#where.kind != TargetKind::Class || rule.r#where.language() == Language::Python,
        "class targets require the Python adapter"
    );
    ensure!(
        rule.r#where.language() != Language::Python
            || matches!(
                rule.r#where.kind,
                TargetKind::Function | TargetKind::Class | TargetKind::File
            ),
        "Python supports only function, class and file targets"
    );
    rule.question.validate()?;
    ensure!(
        !rule.diagnostics.is_empty(),
        "rule needs at least one diagnostic policy"
    );
    for diagnostic in &rule.diagnostics {
        ensure!(
            !diagnostic.message.trim().is_empty(),
            "diagnostic message must not be empty"
        );
        diagnostic.when.validate(rule.question.choices())?;
    }
    Ok(())
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text =
        fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("invalid JSON in {}", path.display()))
}

fn read_config(path: &Path) -> Result<ConfigFile> {
    let text =
        fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let document: ConfigFile = serde_json::from_str(&text)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;
    if document.version == 1 {
        serde_json::from_str::<legacy::ConfigFile>(&text).with_context(|| {
            format!(
                "invalid version-1 JSON in {} (new fields require version 2)",
                path.display()
            )
        })?;
    }
    Ok(document)
}

fn read_rule_file(path: &Path, version: u32) -> Result<RuleFile> {
    let text =
        fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let rules = serde_json::from_str(&text)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;
    if version == 1 {
        serde_json::from_str::<legacy::RuleFile>(&text).with_context(|| {
            format!(
                "invalid version-1 rule JSON in {} (new fields require version 2)",
                path.display()
            )
        })?;
    }
    Ok(rules)
}
