// Fork-specific bounded GAS/AT&T32 tokenizer. Not an assembler validator.
// SPDX-License-Identifier: AGPL-3.0-only
use super::text::{Record, Text};
use crate::config::assembly::{Options, Preprocessing, SlashMode};
use anyhow::{Result, bail, ensure};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Serialize)]
pub struct Region<'a> {
    pub name: String,
    pub name_span: crate::source::Span,
    pub body: Record<'a>,
    pub begin_line: Record<'a>,
    pub end_line: Record<'a>,
    pub begin_comment: Record<'a>,
    pub end_comment: Record<'a>,
}
pub struct Parsed<'a> {
    pub records: Vec<Record<'a>>,
    pub statements: Vec<Record<'a>>,
    pub dependencies: Vec<Record<'a>>,
    pub regions: Vec<Region<'a>>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Macro,
    Repeat,
    AsmIf,
    CppIf,
}
struct Block {
    kind: BlockKind,
    start: usize,
    seen_else: bool,
}
struct Begin<'a> {
    name: String,
    name_span: crate::source::Span,
    line: Record<'a>,
    comment: Record<'a>,
}
struct Scanner<'a> {
    text: Text<'a>,
    options: Options,
    pos: usize,
    line_start: usize,
    records: Vec<Record<'a>>,
    statements: Vec<Record<'a>>,
    dependencies: Vec<Record<'a>>,
    code: Vec<(usize, usize)>,
    blocks: Vec<Block>,
    begin: Option<Begin<'a>>,
    names: BTreeSet<String>,
    regions: Vec<Region<'a>>,
}

pub fn tokenize(source: &str, options: Options) -> Result<Parsed<'_>> {
    options.profile.validate()?;
    let mut scan = Scanner {
        text: Text::new(source),
        options,
        pos: 0,
        line_start: 0,
        records: vec![],
        statements: vec![],
        dependencies: vec![],
        code: vec![],
        blocks: vec![],
        begin: None,
        names: BTreeSet::new(),
        regions: vec![],
    };
    while scan.pos < source.len() {
        scan.next()?;
    }
    scan.flush()?;
    ensure!(
        scan.blocks.is_empty(),
        "unclosed assembly macro/repetition/conditional structure"
    );
    ensure!(scan.begin.is_none(), "unclosed assembly region");
    Ok(Parsed {
        records: scan.records,
        statements: scan.statements,
        dependencies: scan.dependencies,
        regions: scan.regions,
    })
}
fn horizontal(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}
fn ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'$' | b'@' | b'?')
}
fn cpp_header(line: &str) -> Option<&str> {
    let tail = line.strip_prefix('#')?.trim_start_matches([' ', '\t']);
    let head = tail
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()?;
    (matches!(
        head,
        "define"
            | "undef"
            | "include"
            | "if"
            | "ifdef"
            | "ifndef"
            | "elif"
            | "else"
            | "endif"
            | "line"
            | "error"
            | "warning"
            | "pragma"
    ) || (!head.is_empty() && head.bytes().all(|b| b.is_ascii_digit())))
    .then_some(head)
}
impl<'a> Scanner<'a> {
    fn bytes(&self) -> &[u8] {
        self.text.source.as_bytes()
    }
    fn line_end(&self, start: usize) -> usize {
        self.bytes()[start..]
            .iter()
            .position(|b| matches!(b, b'\r' | b'\n'))
            .map_or(self.bytes().len(), |i| start + i)
    }
    fn eol_end(&self, at: usize) -> Result<usize> {
        if at == self.bytes().len() {
            return Ok(at);
        }
        if self.bytes()[at] == b'\r' {
            ensure!(
                self.bytes().get(at + 1) == Some(&b'\n'),
                "unsupported bare CR at byte {at}"
            );
            Ok(at + 2)
        } else {
            Ok(at + 1)
        }
    }
    fn emit(&mut self, kind: &'static str, end: usize, code: bool) {
        if code {
            self.code.push((self.pos, end));
        }
        self.records.push(self.text.record(kind, self.pos, end));
        if let Some(i) = self.text.source[self.pos..end].rfind('\n') {
            self.line_start = self.pos + i + 1;
        }
        self.pos = end;
    }
    fn next(&mut self) -> Result<()> {
        let start = self.pos;
        let b = self.bytes()[start];
        let end = self.line_end(start);
        let standalone = self.bytes()[self.line_start..start]
            .iter()
            .all(|&c| horizontal(c));
        if b == b'#' && standalone {
            let line = &self.text.source[start..end];
            if let Some(header) = cpp_header(line) {
                ensure!(
                    self.options.preprocessing == Preprocessing::CppUnexpanded,
                    "preprocessing_mode_required at byte {start}"
                );
                return self.cpp(header.to_owned());
            }
            ensure!(
                !matches!(line.trim_end_matches([' ', '\t']), "#APP" | "#NO_APP"),
                "unsupported assembler scrub control at byte {start}"
            );
        }
        match b {
            b' ' | b'\t' => {
                let mut e = start + 1;
                while self.bytes().get(e).is_some_and(|&b| horizontal(b)) {
                    e += 1;
                }
                self.emit("whitespace", e, false);
            }
            b'\n' | b'\r' => {
                self.flush()?;
                let e = self.eol_end(start)?;
                self.emit("newline", e, false);
            }
            b';' => {
                self.flush()?;
                self.emit("separator", start + 1, false);
            }
            b'"' => {
                let e = self.string()?;
                self.emit("string", e, true);
            }
            b'\'' => {
                let e = self.character()?;
                self.emit("character", e, true);
            }
            b'#' => {
                if standalone {
                    self.marker(end)?;
                }
                self.emit("comment", end, false);
            }
            b'/' if self.bytes().get(start + 1) == Some(&b'*') => {
                let relative = self.text.source[start + 2..]
                    .find("*/")
                    .ok_or_else(|| anyhow::anyhow!("unterminated block comment at byte {start}"))?;
                let e = start + 2 + relative + 2;
                self.emit("comment", e, false);
            }
            b'/' if self.options.slash_mode == SlashMode::GasDefault => {
                self.emit("comment", end, false)
            }
            b'/' if self.bytes().get(start + 1) == Some(&b'/') => {
                bail!("unsupported // in divide mode at byte {start}")
            }
            b'*' if self.bytes().get(start + 1) == Some(&b'/') => {
                bail!("unmatched block comment terminator at byte {start}")
            }
            b'\\'
                if self
                    .bytes()
                    .get(start + 1)
                    .is_some_and(|b| matches!(b, b'\r' | b'\n')) =>
            {
                let e = self.eol_end(start + 1)?;
                self.emit("continuation", e, false);
            }
            _ if ident(b) => {
                let mut e = start + 1;
                while self.bytes().get(e).is_some_and(|&b| ident(b)) {
                    e += 1;
                }
                self.emit("atom", e, true);
            }
            _ if b.is_ascii_graphic() => self.emit("punctuation", start + 1, true),
            _ => bail!("unsupported assembly character outside literal/comment at byte {start}"),
        }
        Ok(())
    }
    fn escaped(&self, at: usize, single_quote: bool) -> Result<usize> {
        ensure!(
            self.bytes().get(at + 1).is_some_and(|b| matches!(
                b,
                b'\\' | b'\'' | b'"' | b'b' | b'f' | b'n' | b'r' | b't'
            )),
            "unsupported escape at byte {at}"
        );
        ensure!(
            single_quote || self.bytes()[at + 1] != b'\'',
            "unsupported single-quote escape in string at byte {at}"
        );
        Ok(at + 2)
    }
    fn string(&self) -> Result<usize> {
        let mut i = self.pos + 1;
        while i < self.bytes().len() {
            match self.bytes()[i] {
                b'"' => return Ok(i + 1),
                b'\\' => i = self.escaped(i, false)?,
                b'\r' | b'\n' => bail!("unsupported newline in string at byte {i}"),
                _ => i += self.text.source[i..].chars().next().unwrap().len_utf8(),
            }
        }
        bail!("unterminated string at byte {}", self.pos)
    }
    fn character(&self) -> Result<usize> {
        let i = self.pos + 1;
        let c = *self
            .bytes()
            .get(i)
            .ok_or_else(|| anyhow::anyhow!("unterminated character at byte {}", self.pos))?;
        let end = if c == b'\\' {
            self.escaped(i, true)?
        } else {
            ensure!(
                (b' '..=b'~').contains(&c),
                "unsupported character literal at byte {}",
                self.pos
            );
            i + 1
        };
        ensure!(
            self.bytes().get(end).is_none_or(|b| horizontal(*b)
                || matches!(
                    b,
                    b'\r'
                        | b'\n'
                        | b','
                        | b'+'
                        | b'-'
                        | b'*'
                        | b'/'
                        | b'%'
                        | b'&'
                        | b'|'
                        | b'^'
                        | b'~'
                        | b'!'
                        | b'='
                        | b'<'
                        | b'>'
                        | b'('
                        | b')'
                        | b'#'
                        | b';'
                )),
            "unsupported character boundary at byte {end}"
        );
        Ok(end)
    }
    fn cpp(&mut self, header: String) -> Result<()> {
        self.flush()?;
        let start = self.pos;
        let mut end = self.line_end(start);
        loop {
            let continued = end > start && self.bytes()[end - 1] == b'\\';
            let next = self.eol_end(end)?;
            if !continued {
                end = next;
                break;
            }
            ensure!(
                next > end && next < self.bytes().len(),
                "dangling CPP continuation at byte {end}"
            );
            end = self.line_end(next);
        }
        self.condition(&header, BlockKind::CppIf, start, end)?;
        self.dependencies.push(self.text.record("cpp", start, end));
        self.emit("cpp", end, false);
        Ok(())
    }
    fn condition(&mut self, head: &str, kind: BlockKind, start: usize, end: usize) -> Result<()> {
        let name = head.trim_start_matches('.');
        match name {
            "if" | "ifdef" | "ifndef" => self.blocks.push(Block {
                kind,
                start,
                seen_else: false,
            }),
            "else" | "elseif" | "elif" => {
                let block = self.blocks.last_mut().ok_or_else(|| {
                    anyhow::anyhow!("unmatched conditional branch at byte {start}")
                })?;
                ensure!(
                    block.kind == kind && !block.seen_else,
                    "crossed or duplicate conditional branch at byte {start}"
                );
                block.seen_else = name == "else";
            }
            "endif" => self.close(kind, start, end)?,
            _ => {}
        }
        Ok(())
    }
    fn close(&mut self, kind: BlockKind, start: usize, end: usize) -> Result<()> {
        let block = self
            .blocks
            .pop()
            .ok_or_else(|| anyhow::anyhow!("unmatched structural terminator at byte {start}"))?;
        ensure!(
            block.kind == kind,
            "crossed assembly structure at byte {start}"
        );
        let label = match kind {
            BlockKind::Macro => "macro_body",
            BlockKind::Repeat => "repetition_body",
            _ => "conditional_body",
        };
        self.dependencies
            .push(self.text.record(label, block.start, end));
        Ok(())
    }
    fn flush(&mut self) -> Result<()> {
        let code = std::mem::take(&mut self.code);
        if code.is_empty() {
            return Ok(());
        }
        let start = code[0].0;
        let end = code.last().unwrap().1;
        let mut index = 0;
        while index + 1 < code.len()
            && &self.text.source[code[index + 1].0..code[index + 1].1] == ":"
        {
            self.statements
                .push(self.text.record("label", code[index].0, code[index + 1].1));
            index += 2;
        }
        if index == code.len() {
            return Ok(());
        }
        let head = &self.text.source[code[index].0..code[index].1];
        let forbidden = (head.starts_with(".code") && head != ".code32")
            || matches!(
                head,
                ".code16"
                    | ".code64"
                    | ".intel_syntax"
                    | ".altmacro"
                    | ".noaltmacro"
                    | ".cpu"
                    | ".setcpu"
                    | ".arch"
                    | ".machine"
                    | ".syntax"
            );
        ensure!(
            !forbidden,
            "unsupported assembly state directive {head} at byte {start}"
        );
        if head.starts_with(".if") {
            ensure!(
                matches!(head, ".if" | ".ifdef" | ".ifndef"),
                "unsupported_conditional_form {head} at byte {start}"
            );
        }
        match head {
            ".macro" => {
                ensure!(
                    code.len() > index + 1,
                    "macro requires a name at byte {start}"
                );
                ensure!(
                    !self.blocks.iter().any(|b| b.kind == BlockKind::Macro),
                    "nested macro definitions are unsupported"
                );
                self.blocks.push(Block {
                    kind: BlockKind::Macro,
                    start,
                    seen_else: false,
                });
            }
            ".rept" | ".irp" | ".irpc" => {
                ensure!(
                    code.len() > index + 1,
                    "repetition requires operands at byte {start}"
                );
                self.blocks.push(Block {
                    kind: BlockKind::Repeat,
                    start,
                    seen_else: false,
                });
            }
            ".endm" => self.close(BlockKind::Macro, start, end)?,
            ".endr" => self.close(BlockKind::Repeat, start, end)?,
            _ if head.starts_with('.') => self.condition(head, BlockKind::AsmIf, start, end)?,
            _ => {}
        }
        let kind = if head.starts_with('.') {
            "directive"
        } else if code.iter().any(|&(a, b)| &self.text.source[a..b] == "=") {
            "assignment"
        } else {
            "opaque_statement"
        };
        let record = self.text.record(kind, code[index].0, end);
        if kind != "opaque_statement" {
            self.dependencies.push(record.clone());
        }
        self.statements.push(record);
        Ok(())
    }
    fn marker(&mut self, end: usize) -> Result<()> {
        if self
            .blocks
            .iter()
            .any(|b| matches!(b.kind, BlockKind::Macro | BlockKind::Repeat))
        {
            return Ok(());
        }
        let start = self.pos;
        let raw = &self.text.source[start + 1..end];
        if !raw.as_bytes().first().is_some_and(|b| horizontal(*b)) {
            return Ok(());
        }
        let content = raw.trim_start_matches([' ', '\t']);
        let begin = content.starts_with("erislint-region-begin");
        let finish = content.starts_with("erislint-region-end");
        if !begin && !finish {
            return Ok(());
        }
        let keyword = if begin {
            "erislint-region-begin"
        } else {
            "erislint-region-end"
        };
        let tail = &content[keyword.len()..];
        ensure!(
            tail.as_bytes().first().is_some_and(|b| horizontal(*b)),
            "malformed region marker at byte {start}"
        );
        let name = tail.trim_matches([' ', '\t']);
        ensure!(
            !name.is_empty()
                && name.len() <= 64
                && (name.as_bytes()[0].is_ascii_alphabetic() || name.starts_with('_')),
            "invalid region name at byte {start}"
        );
        ensure!(
            name.len() <= 64
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-')),
            "invalid region name or trailing text at byte {start}"
        );
        let name_start = start
            + 1
            + (raw.len() - content.len())
            + keyword.len()
            + (tail.len() - tail.trim_start_matches([' ', '\t']).len());
        let line_end = self.eol_end(end)?;
        let line = self.text.record("marker_line", self.line_start, line_end);
        let comment = self.text.record("marker_comment", start, end);
        if begin {
            ensure!(line_end > end, "region begin requires a newline");
            ensure!(
                self.begin.is_none(),
                "nested assembly regions are unsupported"
            );
            ensure!(
                self.names.insert(name.to_owned()),
                "duplicate assembly region name {name}"
            );
            self.begin = Some(Begin {
                name: name.to_owned(),
                name_span: self.text.span(name_start, name_start + name.len()),
                line,
                comment,
            });
        } else {
            let first = self
                .begin
                .take()
                .ok_or_else(|| anyhow::anyhow!("unmatched region end {name}"))?;
            ensure!(first.name == name, "crossed assembly region names");
            let body = self
                .text
                .record("region", first.line.span.end, self.line_start);
            ensure!(
                !body.source.chars().all(char::is_whitespace),
                "empty assembly region {name}"
            );
            self.regions.push(Region {
                name: first.name,
                name_span: first.name_span,
                body,
                begin_line: first.line,
                end_line: line,
                begin_comment: first.comment,
                end_comment: comment,
            });
        }
        Ok(())
    }
}
