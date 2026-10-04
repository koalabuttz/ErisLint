// Copyright (C) 2026 Eriskii
// SPDX-License-Identifier: AGPL-3.0-only
// See LICENSE for the full license text.

// Fork-specific dispatch extracted from the Rust runner.

use std::{collections::BTreeSet, path::Path};

use anyhow::{Result, ensure};
use ra_ap_syntax::Edition;

use crate::{
    c,
    config::{Config, Language, RustEdition},
    python, rust,
    source::{Target, TargetKind},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Adapter {
    Rust,
    C,
    Python,
}

impl Adapter {
    pub(crate) fn for_path(path: &Path, config: &Config) -> Result<Option<Self>> {
        let relative = path.strip_prefix(&config.root)?;
        let c_selected = config
            .c_filter
            .as_ref()
            .is_some_and(|filter| filter.matches(relative));
        let python_selected = config
            .python_filter
            .as_ref()
            .is_some_and(|filter| filter.matches(relative));
        ensure!(
            !(c_selected && python_selected),
            "path selected by both c_files and python_files: {}",
            relative.display()
        );
        if python_selected {
            ensure!(
                path.extension().is_some_and(|ext| ext == "py"),
                "python_files selected unsupported extension: {} (only .py is supported)",
                relative.display()
            );
            return Ok(Some(Self::Python));
        }
        if c_selected {
            ensure!(
                matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("c" | "h")
                ),
                "c_files selected unsupported extension: {} (only .c and C-mode .h are supported)",
                relative.display()
            );
            return Ok(Some(Self::C));
        }
        Ok(path
            .extension()
            .is_some_and(|extension| extension == "rs")
            .then_some(Self::Rust))
    }

    pub(crate) fn prepare(
        self,
        path: &Path,
        edition: Option<RustEdition>,
    ) -> Result<PreparedAdapter> {
        match self {
            Self::C => Ok(PreparedAdapter::C),
            Self::Python => Ok(PreparedAdapter::Python),
            Self::Rust => Ok(PreparedAdapter::Rust(match edition {
                Some(edition) => edition.into(),
                None => rust::edition_for(path)?,
            })),
        }
    }
}

/// Per-file parser options; the runner retains its directory cache for disk input.
#[derive(Clone, Copy)]
pub(crate) enum PreparedAdapter {
    Rust(Edition),
    C,
    Python,
}

impl PreparedAdapter {
    pub(crate) fn language(self) -> Language {
        match self {
            Self::Rust(_) => Language::Rust,
            Self::C => Language::C,
            Self::Python => Language::Python,
        }
    }

    pub(crate) fn extract(self, source: &str, kinds: &BTreeSet<TargetKind>) -> Result<Vec<Target>> {
        match self {
            Self::Rust(edition) => rust::extract(source, edition, kinds),
            Self::C => c::extract(source, kinds),
            Self::Python => python::extract(source, kinds),
        }
    }
}
