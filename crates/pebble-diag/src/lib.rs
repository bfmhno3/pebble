#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! Diagnostics: errors, warnings, and the pretty-printer that renders them.
//!
//! A [`Diagnostic`] is data. Rendering it requires a [`SourceMap`] so the
//! printer can pull the offending source line and draw carets under it. Keeping
//! the two apart means a diagnostic can be produced deep in the compiler,
//! collected, and only rendered once — which is exactly what the parser does
//! when it recovers from an error and keeps going.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Write as _};

use pebble_span::{SourceMap, Span};

/// How serious a diagnostic is.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl Severity {
    /// A lowercase, stable name for the severity.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A pointer at one span of source, with an optional message.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Label {
    /// Where to point.
    pub span: Span,
    /// What to say about it.
    pub message: Option<String>,
    /// Primary labels get the `^^^^` underline; secondary ones get `----`.
    pub primary: bool,
}

impl Label {
    /// A primary label.
    #[must_use]
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: Some(message.into()),
            primary: true,
        }
    }

    /// A secondary label.
    #[must_use]
    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: Some(message.into()),
            primary: false,
        }
    }
}

/// A single compiler message.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Diagnostic {
    /// Error / warning / note / help.
    pub severity: Severity,
    /// An optional stable code, e.g. `E0001`.
    pub code: Option<String>,
    /// The headline.
    pub message: String,
    /// Pointers into the source.
    pub labels: Vec<Label>,
    /// Trailing `= note:` lines.
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Creates an error with just a headline.
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    /// Creates a warning with just a headline.
    #[must_use]
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(message)
        }
    }

    /// Attaches a stable error code.
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Attaches a primary label.
    #[must_use]
    pub fn with_primary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::primary(span, message));
        self
    }

    /// Attaches a secondary label.
    #[must_use]
    pub fn with_secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::secondary(span, message));
        self
    }

    /// Attaches an unlabeled primary span.
    #[must_use]
    pub fn with_span(mut self, span: Span) -> Self {
        self.labels.push(Label {
            span,
            message: None,
            primary: true,
        });
        self
    }

    /// Appends a `= note:` line.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// The span the printer should focus on: the first primary label, if any.
    #[must_use]
    pub fn primary_span(&self) -> Option<Span> {
        self.labels
            .iter()
            .find(|label| label.primary)
            .map(|label| label.span)
    }

    /// Renders the diagnostic against a source map into a multi-line string.
    ///
    /// The output mimics the `rustc` style:
    ///
    /// ```text
    /// error[E0001]: unexpected token
    ///   --> demo.pebble:4:13
    ///    |
    ///  4 |     let x = ;
    ///    |             ^ expected an expression
    /// ```
    #[must_use]
    pub fn render(&self, sources: &SourceMap) -> String {
        let mut out = String::new();
        self.write_header(&mut out);

        // Stable order so snapshots do not depend on construction order.
        let mut labels: Vec<&Label> = self.labels.iter().collect();
        labels.sort_by_key(|label| (label.span.source, label.span.start));

        for label in labels {
            let Some(file) = sources.get(label.span.source) else {
                continue;
            };
            let pos = file.position(label.span.start);
            let line_no = pos.line;
            let width = digits(line_no);
            let pad = " ".repeat(width);

            let _ = writeln!(out, "{pad}--> {}:{}:{}", file.name(), line_no, pos.column);
            let _ = writeln!(out, "{pad} |");

            let line_text = file.line(line_no).unwrap_or("");
            let _ = writeln!(out, "{line_no:>width$} | {line_text}");

            let (caret_col, caret_len) = caret_geometry(file, label.span, line_no, line_text);
            let marker = if label.primary { '^' } else { '-' };
            let carets: String = core::iter::repeat_n(marker, caret_len).collect();
            let lead = " ".repeat(caret_col);
            match &label.message {
                Some(message) => {
                    let _ = writeln!(out, "{pad} | {lead}{carets} {message}");
                }
                None => {
                    let _ = writeln!(out, "{pad} | {lead}{carets}");
                }
            }
            let _ = writeln!(out, "{pad} |");
        }

        for note in &self.notes {
            let _ = writeln!(out, "  = note: {note}");
        }

        out
    }

    fn write_header(&self, out: &mut String) {
        let _ = write!(out, "{}", self.severity);
        if let Some(code) = &self.code {
            let _ = write!(out, "[{code}]");
        }
        let _ = writeln!(out, ": {}", self.message);
    }
}

/// `core::error::Error` is available even in `no_std` since Rust 1.81.
impl core::error::Error for Diagnostic {}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(code) = &self.code {
            write!(f, "{}[{}]: {}", self.severity, code, self.message)
        } else {
            write!(f, "{}: {}", self.severity, self.message)
        }
    }
}

/// Something that accepts diagnostics.
///
/// This is the trait that lets the parser be generic over "where errors go":
/// a `Vec<Diagnostic>` collects them, while [`SilentSink`] throws them away.
pub trait DiagnosticSink {
    /// Records one diagnostic.
    fn emit(&mut self, diagnostic: Diagnostic);

    /// Convenience: emit an error built from a span.
    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.emit(Diagnostic::error(message).with_span(span));
    }
}

impl DiagnosticSink for Vec<Diagnostic> {
    fn emit(&mut self, diagnostic: Diagnostic) {
        self.push(diagnostic);
    }
}

/// A sink that drops everything; handy for tests and "just parse it" callers.
#[derive(Default, Debug)]
pub struct SilentSink;

impl DiagnosticSink for SilentSink {
    fn emit(&mut self, _diagnostic: Diagnostic) {}
}

/// Either a value or the diagnostics that stopped us producing one.
pub type DiagResult<T> = Result<T, Diagnostic>;

/// Number of decimal digits in `n` (at least 1).
fn digits(n: u32) -> usize {
    let mut n = n;
    let mut count = 1;
    while n >= 10 {
        n /= 10;
        count += 1;
    }
    count
}

/// Computes the caret start column and length, in characters, for a span.
fn caret_geometry(
    file: &pebble_span::SourceFile,
    span: Span,
    line_no: u32,
    line_text: &str,
) -> (usize, usize) {
    let line_start = file.line_start(line_no).unwrap_or(0);
    let byte_in_line = span.start.saturating_sub(line_start) as usize;
    let byte_in_line = byte_in_line.min(line_text.len());

    // Character columns, not byte columns, so CJK lines line up.
    let caret_col = line_text[..byte_in_line].chars().count();

    let line_end = line_start + line_text.len() as u32;
    let covered_end = span.end.min(line_end).max(span.start);
    let covered = (covered_end - span.start) as usize;
    let slice_end = (byte_in_line + covered).min(line_text.len());
    let caret_len = line_text[byte_in_line..slice_end].chars().count().max(1);

    (caret_col, caret_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_single_caret_line() {
        let mut sources = SourceMap::new();
        let id = sources.add("demo.pebble", "let x = 1;\nlet y = ;\n");
        let span = Span::new(id, 19, 19);
        let diag = Diagnostic::error("expected an expression")
            .with_code("E0001")
            .with_primary(span, "found `;`");

        let rendered = diag.render(&sources);
        assert!(
            rendered.starts_with("error[E0001]: expected an expression\n"),
            "header must be on its own line:\n{rendered}"
        );
        assert!(rendered.contains("--> demo.pebble:2:9"));
        assert!(rendered.contains("^ found `;`"));
    }

    #[test]
    fn caret_length_tracks_span_length() {
        let mut sources = SourceMap::new();
        let id = sources.add("demo.pebble", "let abcdef = 1;\n");
        // Span covering `abcdef` (bytes 4..10).
        let diag = Diagnostic::error("bad name").with_span(Span::new(id, 4, 10));
        let rendered = diag.render(&sources);
        assert!(rendered.contains("^^^^^^"), "got:\n{rendered}");
    }

    #[test]
    fn notes_are_rendered() {
        let mut sources = SourceMap::new();
        let id = sources.add("demo.pebble", "x\n");
        let diag = Diagnostic::error("nope")
            .with_span(Span::new(id, 0, 1))
            .with_note("did you mean `y`?");
        assert!(diag.render(&sources).contains("= note: did you mean `y`?"));
    }

    #[test]
    fn sink_collects_errors() {
        let mut sink = Vec::new();
        sink.error(Span::DUMMY, "boom");
        assert_eq!(sink.len(), 1);
    }
}
