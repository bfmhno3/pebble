use pebble_diag::{Diagnostic, DiagnosticSink, Severity};
use pebble_span::{SourceMap, Span};

struct ErrorCounter {
    errors: usize,
}

impl DiagnosticSink for ErrorCounter {
    fn emit(&mut self, diagnostic: Diagnostic) {
        if diagnostic.severity == Severity::Error {
            self.errors += 1;
        }
    }
}

#[test]
fn help_renders_with_its_own_word() {
    let diag = Diagnostic {
        severity: Severity::Help,
        ..Diagnostic::error("try `let`")
    };
    let rendered = diag.render(&SourceMap::new());
    assert_eq!(rendered, "help: try `let`\n");
}

#[test]
fn two_labels_on_two_lines() {
    let mut sources = SourceMap::new();
    let id = sources.add("demo.pebble", "let a = 1;\nlet b = a + c;\n");
    let diag = Diagnostic::error("undefined variable `c`")
        .with_primary(Span::new(id, 23, 24), "not found")
        .with_secondary(Span::new(id, 4, 5), "defined here");
    println!("{}", diag.render(&sources));
}

#[test]
fn counting_sink_counts_only_errors() {
    let mut sink = ErrorCounter { errors: 0 };
    sink.error(Span::DUMMY, "boom");
    sink.emit(Diagnostic::warning("hmm"));
    sink.error(Span::DUMMY, "again");
    assert_eq!(sink.errors, 2);
}
