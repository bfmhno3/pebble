#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
//! Source offsets, spans, and source maps.
//!
//! Everything in the front end of the compiler talks about positions in terms
//! of *byte offsets into a source file*. Those offsets are grouped into
//! [`Span`]s, and [`SourceMap`] is the thing that turns an offset back into a
//! human position (`file.pebble:12:7`).
//!
//! The crate is `#![no_std]` on purpose: it only needs `core` and `alloc`, so
//! the same code runs on a microcontroller (see `pebble-nostd`). That is also
//! why you will see `alloc::string::String` instead of `std::string::String`.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Identifies one source file inside a [`SourceMap`].
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId(pub u32);

impl fmt::Debug for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "source#{}", self.0)
    }
}

/// A half-open byte range `[start, end)` inside a single source file.
///
/// `Span` is `Copy`: it is three small integers, so passing it around is free
/// and never moves any source text.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub source: SourceId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    /// A span that points at nothing; useful as a placeholder.
    pub const DUMMY: Span = Span {
        source: SourceId(0),
        start: 0,
        end: 0,
    };

    /// Builds a span from a source id and a byte range.
    #[must_use]
    pub const fn new(source: SourceId, start: u32, end: u32) -> Self {
        Self { source, start, end }
    }

    /// An empty span located at `at`.
    #[must_use]
    pub const fn empty(source: SourceId, at: u32) -> Self {
        Self {
            source,
            start: at,
            end: at,
        }
    }

    /// Number of bytes covered by the span.
    #[must_use]
    pub const fn len(self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    /// Whether the span covers zero bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }

    /// The smallest span covering both `self` and `other`.
    ///
    /// Both spans must belong to the same source file; the compiler guarantees
    /// this, and a debug build will panic if it is ever violated.
    #[must_use]
    pub fn merge(self, other: Span) -> Span {
        debug_assert_eq!(
            self.source, other.source,
            "tried to merge spans from different files"
        );
        Span {
            source: self.source,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Whether `offset` falls inside the half-open range.
    #[must_use]
    pub const fn contains(self, offset: u32) -> bool {
        self.start <= offset && offset < self.end
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// A one-based line and column, counted in Unicode scalar values.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// One file of source text plus the offsets at which each line starts.
#[derive(Clone)]
pub struct SourceFile {
    id: SourceId,
    name: String,
    text: String,
    /// Byte offset of the first character of every line. Always starts with 0.
    line_starts: Vec<u32>,
}

impl SourceFile {
    fn new(id: SourceId, name: String, text: String) -> Self {
        let line_starts = compute_line_starts(&text);
        Self {
            id,
            name,
            text,
            line_starts,
        }
    }

    /// The id handed out by the owning [`SourceMap`].
    #[must_use]
    pub fn id(&self) -> SourceId {
        self.id
    }

    /// A human-readable name, e.g. a path.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The raw source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Number of lines. A file always has at least one (possibly empty) line.
    #[must_use]
    pub fn line_count(&self) -> u32 {
        self.line_starts.len() as u32
    }

    /// Byte offset at which one-based `line` starts.
    #[must_use]
    pub fn line_start(&self, line: u32) -> Option<u32> {
        let index = line.checked_sub(1)? as usize;
        self.line_starts.get(index).copied()
    }

    /// Zero-based index of the line containing `offset`.
    #[must_use]
    pub fn line_index(&self, offset: u32) -> u32 {
        // `line_starts[0] == 0 <= offset` always holds, so the result is >= 1.
        let count = self.line_starts.partition_point(|&start| start <= offset);
        (count - 1) as u32
    }

    /// The text of one-based `line`, without its trailing line terminator.
    #[must_use]
    pub fn line(&self, line: u32) -> Option<&str> {
        let index = line.checked_sub(1)? as usize;
        let start = *self.line_starts.get(index)? as usize;
        let end = self
            .line_starts
            .get(index + 1)
            .map_or(self.text.len(), |&next| next as usize);
        let raw = &self.text[start..end];
        Some(strip_line_ending(raw))
    }

    /// Converts a byte offset into a one-based [`Position`].
    #[must_use]
    pub fn position(&self, offset: u32) -> Position {
        let index = self.line_index(offset);
        let line_start = self.line_starts[index as usize];
        let head = &self.text[line_start as usize..(offset as usize).min(self.text.len())];
        Position {
            line: index + 1,
            // +1 because columns are one-based.
            column: head.chars().count() as u32 + 1,
        }
    }
}

impl fmt::Debug for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourceFile")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("lines", &self.line_count())
            .field("bytes", &self.text.len())
            .finish()
    }
}

fn compute_line_starts(text: &str) -> Vec<u32> {
    let mut starts = Vec::with_capacity(text.len() / 24 + 1);
    starts.push(0);
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index as u32 + 1);
        }
    }
    starts
}

fn strip_line_ending(line: &str) -> &str {
    let line = line.strip_suffix('\n').unwrap_or(line);
    line.strip_suffix('\r').unwrap_or(line)
}

/// Anything that knows the source range it came from.
///
/// The AST implements this for every node; the `pebble-macros` crate can derive
/// it, and `pebble-span` provides the blanket cases.
pub trait Spanned {
    /// The node's source range.
    fn span(&self) -> Span;
}

impl Spanned for Span {
    fn span(&self) -> Span {
        *self
    }
}

/// Owns every source file and hands out [`SourceId`]s.
#[derive(Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    /// Creates an empty map.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new file and returns its id.
    pub fn add(&mut self, name: impl Into<String>, text: impl Into<String>) -> SourceId {
        let id = SourceId(self.files.len() as u32);
        self.files
            .push(SourceFile::new(id, name.into(), text.into()));
        id
    }

    /// Looks up a file by id, returning `None` for an unknown id.
    #[must_use]
    pub fn get(&self, id: SourceId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }

    /// Number of registered files.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether no files have been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

impl core::ops::Index<SourceId> for SourceMap {
    type Output = SourceFile;

    /// Looks up a file by id, panicking on an unknown id.
    ///
    /// Ids only ever come from this map, so a bad id is a bug in the compiler,
    /// not something the user did. Use [`SourceMap::get`] for a fallible lookup.
    ///
    /// # Panics
    ///
    /// Panics when `id` did not come from this map.
    fn index(&self, id: SourceId) -> &SourceFile {
        &self.files[id.0 as usize]
    }
}

impl fmt::Debug for SourceMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourceMap")
            .field("files", &self.files.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_starts_include_a_line_after_a_trailing_newline() {
        let map_file = SourceFile::new(SourceId(0), "t".into(), "a\nb\n".into());
        assert_eq!(map_file.line_count(), 3);
        assert_eq!(map_file.line(1), Some("a"));
        assert_eq!(map_file.line(2), Some("b"));
        assert_eq!(map_file.line(3), Some(""));
    }

    #[test]
    fn handles_crlf() {
        let file = SourceFile::new(SourceId(0), "t".into(), "a\r\nb".into());
        assert_eq!(file.line(1), Some("a"));
        assert_eq!(file.line(2), Some("b"));
    }

    #[test]
    fn position_counts_characters_not_bytes() {
        // Each CJK character is three bytes; the second line's "c" is the 5th
        // column even though it sits at byte offset 8.
        let file = SourceFile::new(SourceId(0), "t".into(), "ab\n中c".into());
        let offset = file.text().find('c').unwrap() as u32;
        let pos = file.position(offset);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.column, 2);
    }

    #[test]
    fn merge_covers_both() {
        let a = Span::new(SourceId(0), 2, 5);
        let b = Span::new(SourceId(0), 7, 9);
        assert_eq!(a.merge(b), Span::new(SourceId(0), 2, 9));
    }

    #[test]
    #[should_panic(expected = "different files")]
    fn merge_across_files_is_a_bug() {
        let a = Span::new(SourceId(0), 0, 1);
        let b = Span::new(SourceId(1), 0, 1);
        let _ = a.merge(b);
    }

    #[test]
    fn source_map_assigns_sequential_ids() {
        let mut map = SourceMap::new();
        let first = map.add("a.pebble", "");
        let second = map.add("b.pebble", "");
        assert_eq!(first, SourceId(0));
        assert_eq!(second, SourceId(1));
        assert_eq!(map[second].name(), "b.pebble");
    }
}
