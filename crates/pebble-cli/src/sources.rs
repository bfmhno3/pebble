//! Loading source text and turning diagnostics into printable text.
//!
//! Every subcommand works the same way: read a file into a [`SourceMap`], hand
//! the text and its id to the lexer/parser/compiler, then render whatever came
//! back against that same map so the carets land on the right characters.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use pebble_diag::{Diagnostic, Severity};
use pebble_span::{SourceId, SourceMap};
use pebble_value::RuntimeError;

/// One registered source file plus the map that owns it.
pub(crate) struct Source {
    /// Maps spans back to the text they came from.
    pub(crate) map: SourceMap,
    /// The id the text was registered under.
    pub(crate) id: SourceId,
}

impl Source {
    /// Reads `path` and registers its text in a fresh map.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read as UTF-8 text.
    pub(crate) fn load(path: &Path) -> Result<Self> {
        let text =
            fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
        Ok(Self::from_text(path.display().to_string(), text))
    }

    /// Registers `text` under `name` in a fresh map.
    pub(crate) fn from_text(name: impl Into<String>, text: impl Into<String>) -> Self {
        let mut map = SourceMap::new();
        let id = map.add(name, text);
        Self { map, id }
    }

    /// The registered text.
    #[must_use]
    pub(crate) fn text(&self) -> &str {
        self.map[self.id].text()
    }
}

/// Whether any diagnostic is an error (as opposed to a warning).
#[must_use]
pub(crate) fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

/// Renders every diagnostic, one block after another.
#[must_use]
pub(crate) fn render(diagnostics: &[Diagnostic], sources: &SourceMap) -> String {
    let mut out = String::new();
    for diagnostic in diagnostics {
        out.push_str(&diagnostic.render(sources));
    }
    out
}

/// Renders a runtime error, followed by its stack trace.
#[must_use]
pub(crate) fn render_runtime(error: &RuntimeError, sources: &SourceMap) -> String {
    let mut out = error.diagnostic.render(sources);
    for frame in &error.trace {
        match sources.get(frame.span.source) {
            Some(file) => {
                let position = file.position(frame.span.start);
                out.push_str(&format!(
                    "  in `{}` at {}:{}:{}\n",
                    frame.function,
                    file.name(),
                    position.line,
                    position.column
                ));
            }
            None => out.push_str(&format!("  in `{}`\n", frame.function)),
        }
    }
    out
}

/// Prints every diagnostic to stderr; returns `true` if any of them is an error.
pub(crate) fn emit(diagnostics: &[Diagnostic], sources: &SourceMap) -> bool {
    eprint!("{}", render(diagnostics, sources));
    has_errors(diagnostics)
}

/// Prints a runtime error and its stack trace to stderr.
pub(crate) fn emit_runtime(error: &RuntimeError, sources: &SourceMap) {
    eprint!("{}", render_runtime(error, sources));
}
