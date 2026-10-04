// Fork-specific bounded assembly configuration.
// SPDX-License-Identifier: AGPL-3.0-only
use super::{FileFilter, Rule};
use crate::policy::Condition;
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const UNCERTAINTY: &str = "insufficient_context";
pub const UNCERTAINTY_DESCRIPTION: &str =
    "Required context is missing, unknown, or unsupported; no substantive judgment can be made.";
pub const GUIDANCE: &str = "Assembly source review is incomplete. Select insufficient_context when missing, unknown or unsupported assembler, macro, preprocessing or CPU state prevents judgment. Do not infer assembler acceptance or instruction correctness.";
pub const INCONCLUSIVE: &str = "Assembly review inconclusive: insufficient context.";

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    X86GasAtt32,
    MosLlvmC64,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Preprocessing {
    None,
    CppUnexpanded,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum SlashMode {
    GasDefault,
    Divide,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Options {
    pub profile: Profile,
    pub preprocessing: Preprocessing,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slash_mode: Option<SlashMode>,
}
impl Options {
    pub fn validate(self) -> Result<()> {
        match self.profile {
            Profile::X86GasAtt32 => ensure!(
                self.slash_mode.is_some(),
                "x86-gas-att32 requires explicit slash_mode"
            ),
            Profile::MosLlvmC64 => ensure!(
                self.slash_mode.is_none(),
                "mos-llvm-c64 does not accept GAS slash_mode"
            ),
        }
        Ok(())
    }
}
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub files: Vec<String>,
    pub profile: Profile,
    pub preprocessing: Preprocessing,
    pub slash_mode: Option<SlashMode>,
}
pub struct CompiledSource {
    pub filter: FileFilter,
    pub options: Options,
}
impl Source {
    pub fn compile(self) -> Result<CompiledSource> {
        ensure!(
            !self.files.is_empty(),
            "assembly files must contain at least one pattern"
        );
        let options = Options {
            profile: self.profile,
            preprocessing: self.preprocessing,
            slash_mode: self.slash_mode,
        };
        options.validate()?;
        Ok(CompiledSource {
            filter: FileFilter::new(&self.files, &[])?,
            options,
        })
    }
}
pub fn validate_rule(rule: &Rule) -> Result<()> {
    rule.r#where
        .profile
        .ok_or_else(|| anyhow::anyhow!("assembly rule requires explicit profile"))?;
    ensure!(
        rule.question
            .choices()
            .get(UNCERTAINTY)
            .is_some_and(|description| description == UNCERTAINTY_DESCRIPTION),
        "assembly requires the exact insufficient_context choice and description"
    );
    for policy in &rule.diagnostics {
        ensure!(
            policy
                .when
                .choice
                .as_deref()
                .is_some_and(|choice| choice != UNCERTAINTY),
            "assembly diagnostics require a substantive top-level when.choice"
        );
        ensure!(
            !mentions_uncertainty(&policy.when),
            "assembly diagnostic conditions cannot reference insufficient_context"
        );
    }
    Ok(())
}
fn mentions_uncertainty(condition: &Condition) -> bool {
    condition.choice.as_deref() == Some(UNCERTAINTY)
        || condition
            .probability
            .as_ref()
            .is_some_and(|p| p.choice == UNCERTAINTY)
        || [&condition.all, &condition.any]
            .into_iter()
            .flatten()
            .flatten()
            .any(mentions_uncertainty)
}
