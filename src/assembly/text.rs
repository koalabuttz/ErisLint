// Fork-specific lossless source records. No assembler interpretation.
// SPDX-License-Identifier: AGPL-3.0-only
use crate::source::Span;
use serde::Serialize;

pub struct Text<'a> {
    pub source: &'a str,
    starts: Vec<usize>,
}
impl<'a> Text<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            starts: std::iter::once(0)
                .chain(source.match_indices('\n').map(|(i, _)| i + 1))
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
    pub fn span(&self, start: usize, end: usize) -> Span {
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
    pub fn record(&self, kind: &'static str, start: usize, end: usize) -> Record<'a> {
        Record {
            kind,
            source: &self.source[start..end],
            span: self.span(start, end),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Record<'a> {
    pub kind: &'static str,
    pub source: &'a str,
    pub span: Span,
}
