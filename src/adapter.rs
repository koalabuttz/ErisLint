// Copyright (C) 2026 Eriskii
// SPDX-License-Identifier: AGPL-3.0-only
// See LICENSE for the full license text.

// Fork-specific dispatch extracted from the Rust runner.

use std::{collections::BTreeSet, path::Path};

use anyhow::Result;
use ra_ap_syntax::Edition;

use crate::{
    config::RustEdition,
    rust,
    source::{Target, TargetKind},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Adapter {
    Rust,
}

impl Adapter {
    pub(crate) fn for_path(path: &Path) -> Option<Self> {
        path.extension()
            .is_some_and(|extension| extension == "rs")
            .then_some(Self::Rust)
    }

    pub(crate) fn prepare(
        self,
        path: &Path,
        edition: Option<RustEdition>,
    ) -> Result<PreparedAdapter> {
        match self {
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
}

impl PreparedAdapter {
    pub(crate) fn extract(self, source: &str, kinds: &BTreeSet<TargetKind>) -> Result<Vec<Target>> {
        match self {
            Self::Rust(edition) => rust::extract(source, edition, kinds),
        }
    }
}
