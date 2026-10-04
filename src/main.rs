// Copyright (C) 2026 Eriskii
// SPDX-License-Identifier: AGPL-3.0-only
// See LICENSE for the full license text.

use std::{
    env,
    io::{self, IsTerminal, Read, Write},
    num::NonZeroUsize,
    path::PathBuf,
    process::ExitCode,
};

use anyhow::{Result, anyhow};
use clap::{Parser, ValueEnum};
use erislint::{
    config::{Config, ConfigFile, RuleFile, legacy, v2},
    jev::JevClient,
    output::{TextOptions, TextStyle, write_text},
    runner::Plan,
};
use serde::Serialize;

#[derive(Parser)]
#[command(version, about = "Evaluate configurable Rust lint rules with Jev")]
struct Cli {
    /// Rust files or directories to lint. Defaults to the configuration directory.
    paths: Vec<PathBuf>,
    /// Read an unsaved snapshot from stdin, using this existing Rust file's identity.
    #[arg(long, conflicts_with_all = ["paths", "check_config", "schema"])]
    stdin_file: Option<PathBuf>,
    /// Evaluate only the function whose name starts at this UTF-8 byte offset.
    #[arg(long, requires = "stdin_file")]
    target_start: Option<usize>,
    /// Use an explicit configuration instead of discovering erislint.json.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Validate configuration and referenced rule files without calling Jev.
    #[arg(long, conflicts_with_all = ["dry_run", "schema"])]
    check_config: bool,
    /// Print AST inputs and Jev requests as JSON without reading jev_key or calling the API.
    #[arg(long, conflicts_with = "schema")]
    dry_run: bool,
    /// Print the configuration or rule-file JSON Schema.
    #[arg(long, value_enum)]
    schema: Option<SchemaKind>,
    /// Diagnostic output format.
    #[arg(long, value_enum, default_value = "text")]
    format: Format,
    /// Include every option's probability in text output.
    #[arg(long)]
    all_answers: bool,
    /// Show only error diagnostics; exit-status rules still apply to the full run.
    #[arg(long, conflicts_with = "all_answers")]
    errors_only: bool,
    /// Colorize text diagnostics. Auto uses color only in a terminal.
    #[arg(long, value_enum, default_value = "auto")]
    color: Color,
    /// Maximum number of concurrent Jev requests.
    #[arg(long, default_value = "64")]
    jobs: NonZeroUsize,
    /// Exit unsuccessfully when warnings are emitted, too.
    #[arg(long)]
    deny_warnings: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Compact,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum Color {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, ValueEnum)]
enum SchemaKind {
    Config,
    Rule,
    ConfigV2,
    RuleV2,
    ConfigV3,
    RuleV3,
}

#[tokio::main]
async fn main() -> ExitCode {
    finish(run(Cli::parse()).await)
}

fn finish(result: Result<u8>) -> ExitCode {
    match result {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("erislint: {error:#}");
            ExitCode::from(2)
        }
    }
}

async fn run(cli: Cli) -> Result<u8> {
    if let Some(kind) = cli.schema {
        let schema = match kind {
            SchemaKind::Config => schemars::schema_for!(legacy::ConfigFile),
            SchemaKind::Rule => schemars::schema_for!(legacy::RuleFile),
            SchemaKind::ConfigV2 => schemars::schema_for!(v2::ConfigFile),
            SchemaKind::RuleV2 => schemars::schema_for!(v2::RuleFile),
            SchemaKind::ConfigV3 => schemars::schema_for!(ConfigFile),
            SchemaKind::RuleV3 => schemars::schema_for!(RuleFile),
        };
        write_json(&schema)?;
        return Ok(0);
    }
    let path = match cli.config {
        Some(path) => path,
        None => Config::discover(&env::current_dir()?)?,
    };
    let config = Config::load(&path)?;
    if cli.check_config {
        match cli.format {
            Format::Text | Format::Compact => println!(
                "Configuration valid: {} rules ({})",
                config.rules.len(),
                config.path.display()
            ),
            Format::Json => write_json(
                &serde_json::json!({ "config": config.path, "rules": config.rules.len(), "valid": true }),
            )?,
        }
        return Ok(0);
    }
    let mut plan = if let Some(path) = cli.stdin_file {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source)?;
        Plan::from_source(&config, &path, &source)?
    } else {
        Plan::build(&config, &cli.paths)?
    };
    if let Some(start) = cli.target_start {
        plan.select_function(start)?;
    }
    if cli.dry_run {
        write_json(&plan)?;
        return Ok(0);
    }
    let mut report =
        if plan.evaluations.is_empty() {
            plan.empty_report()
        } else {
            let key = env::var("jev_key").map_err(|_| anyhow!(
            "set the jev_key environment variable to your TypeSafe API key (or use --dry-run)"
        ))?;
            let client = JevClient::new(&key)?;
            if io::stderr().is_terminal() {
                if config.has_source_adapters() {
                    eprintln!("Checking {} source files...", plan.files);
                } else {
                    eprintln!("Checking {} Rust files...", plan.files);
                }
            }
            plan.run(&config, &client, cli.jobs).await?
        };
    let exit_code = report_status(&mut report, cli.deny_warnings, cli.errors_only);
    match cli.format {
        Format::Text | Format::Compact => write_text(
            io::stdout().lock(),
            &report,
            &plan,
            TextOptions {
                style: if matches!(cli.format, Format::Compact) {
                    TextStyle::Compact
                } else {
                    TextStyle::Source
                },
                errors_only: cli.errors_only,
                all_answers: cli.all_answers,
                color: match cli.color {
                    Color::Auto => io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none(),
                    Color::Always => true,
                    Color::Never => false,
                },
            },
        )?,
        Format::Json => write_json(&report)?,
    }
    Ok(exit_code)
}

fn report_status(
    report: &mut erislint::runner::Report,
    deny_warnings: bool,
    errors_only: bool,
) -> u8 {
    let exit_code = u8::from(report.errors > 0 || (deny_warnings && report.warnings > 0));
    if errors_only {
        report.retain_errors();
    }
    exit_code
}

fn write_json(value: &impl Serialize) -> Result<()> {
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, value)?;
    writeln!(stdout)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use erislint::{
        config::assembly::{INCONCLUSIVE, UNCERTAINTY_DESCRIPTION},
        runner::{diagnostics, rule_answers},
    };
    use serde_json::json;

    // Fork-specific: provider failures must never take the semantic warning path.
    #[test]
    fn response_validation_diagnostics_and_exit_status_are_distinct() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("sample.rs"), "fn example() {}\n").unwrap();
        let config_path = project.path().join("erislint.json");
        let value = json!({"rules":[{"id":"review","where":{"kind":"function"},
        "question":{"type":"choice","instructions":"Review the local observation.",
            "criteria":{"supported":"Supported","concern":"Concern","not_applicable":"Not applicable","insufficient_context":"Missing context"}},
        "diagnostics":[
            {"when":{"choice":"concern","min_confidence":0.65},"level":"warn","message":"Review {name}"},
            {"when":{"choice":"insufficient_context"},"level":"warn","message":"Inconclusive {name}"}
        ]}]});
        std::fs::write(&config_path, serde_json::to_vec(&value).unwrap()).unwrap();
        let config = Config::load(&config_path).unwrap();
        let plan = Plan::build(&config, &[]).unwrap();
        let evaluation = &plan.evaluations[0];
        for (choice, confidence, probabilities, warning) in [
            ("insufficient_context", 0.2, [0.2, 0.2, 0.2, 0.4], true),
            ("concern", 0.65, [0.1, 0.8, 0.05, 0.05], true),
            ("concern", 0.64, [0.1, 0.8, 0.05, 0.05], false),
            ("supported", 0.8, [0.9, 0.05, 0.03, 0.02], false),
        ] {
            for (deny, errors_only) in [(false, false), (true, false), (false, true), (true, true)]
            {
                let [supported, concern, not_applicable, insufficient_context] = probabilities;
                let response = serde_json::from_value(json!({"model":"offline-mock","answers":{"review":{
                    "type":"choice","choice":choice,"confidence":confidence,
                    "probabilities":{"supported":supported,"concern":concern,"not_applicable":not_applicable,"insufficient_context":insufficient_context}
                }}})).unwrap();
                let mut report = plan.empty_report();
                report.answers = rule_answers(evaluation, &response);
                report.diagnostics = diagnostics(&config, evaluation, response).unwrap();
                report.warnings = report.diagnostics.len();
                assert_eq!(report.warnings, usize::from(warning));
                let status = report_status(&mut report, deny, errors_only);
                assert_eq!(
                    finish(Ok(status)),
                    ExitCode::from(u8::from(deny && warning))
                );
                let mut rendered = Vec::new();
                write_text(
                    &mut rendered,
                    &report,
                    &plan,
                    TextOptions {
                        style: TextStyle::Compact,
                        errors_only,
                        all_answers: false,
                        color: false,
                    },
                )
                .unwrap();
                let rendered = String::from_utf8(rendered).unwrap();
                assert_eq!(
                    rendered.contains("Inconclusive example"),
                    choice == "insufficient_context" && !errors_only
                );
                assert_eq!(
                    rendered.contains("Review example"),
                    choice == "concern" && warning && !errors_only
                );
            }
        }
        for choice in ["supported", "insufficient_context"] {
            let response = serde_json::from_value(json!({"model":"offline-mock","answers":{"review":{
                "type":"choice","choice":choice,"confidence":0.2,
                "probabilities":{"supported":0.2,"concern":0.4,"not_applicable":0.2,"insufficient_context":0.2}
            }}})).unwrap();
            let result = diagnostics(&config, evaluation, response).map(|_| 0);
            assert!(
                result
                    .as_ref()
                    .unwrap_err()
                    .to_string()
                    .contains("not a maximum")
            );
            assert_eq!(finish(result), ExitCode::from(2));
        }
    }

    #[test]
    fn mock_assembly_uncertainty_obeys_cli_warning_and_display_flags() {
        for profile in ["x86-gas-att32", "mos-llvm-c64"] {
            for setting in ["warn", "error"] {
                let project = tempfile::tempdir().unwrap();
                std::fs::write(project.path().join("a.s"), "nop\n").unwrap();
                let mut source = json!({"files":["*.s"],"profile":profile,"preprocessing":"none"});
                if profile == "x86-gas-att32" {
                    source["slash_mode"] = json!("gas-default");
                }
                let config_path = project.path().join("erislint.json");
                let value = json!({"version":3,"include":["*.s"],"assembly_sources":[source],
                    "rules":[{"id":"review","where":{"language":"assembly","profile":profile,"kind":"file"},
                    "question":{"type":"choice","instructions":"Review source intent.","criteria":{"good":"Clear intent","insufficient_context":UNCERTAINTY_DESCRIPTION}},
                    "diagnostics":[{"when":{"choice":"good"},"level":"error","message":"Substantive policy"}]}],
                    "overrides":[{"files":["*.s"],"rules":{"review":setting}}]});
                std::fs::write(&config_path, serde_json::to_vec(&value).unwrap()).unwrap();
                let config = Config::load(&config_path).unwrap();
                let plan = Plan::build(&config, &[]).unwrap();
                let evaluation = &plan.evaluations[0];
                for (flags, expected_exit, hidden) in [
                    (vec![], 0, false),
                    (vec!["--deny-warnings"], 1, false),
                    (vec!["--errors-only"], 0, true),
                    (vec!["--deny-warnings", "--errors-only"], 1, true),
                ] {
                    let cli =
                        Cli::try_parse_from(std::iter::once("erislint").chain(flags)).unwrap();
                    let response = serde_json::from_value(json!({"model":"offline-mock","answers":{"review":{"type":"choice","choice":"insufficient_context","confidence":0.99,"probabilities":{"good":0.01,"insufficient_context":0.99}}}})).unwrap();
                    let mut report = plan.empty_report();
                    report.answers = rule_answers(evaluation, &response);
                    report.diagnostics = diagnostics(&config, evaluation, response).unwrap();
                    assert_eq!(report.diagnostics.len(), 1);
                    assert_eq!(report.diagnostics[0].message, INCONCLUSIVE);
                    report.warnings = 1;
                    assert_eq!(
                        report_status(&mut report, cli.deny_warnings, cli.errors_only),
                        expected_exit
                    );
                    assert_eq!(report.diagnostics.is_empty(), hidden);
                    assert_eq!(report.answers.is_empty(), hidden);
                    assert_eq!(report.warnings, usize::from(!hidden));
                    assert_eq!(report.errors, 0);
                }
            }
        }
    }
}
