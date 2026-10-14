#[derive(Copy, Clone)]
enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

fn as_str(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    }
}

fn main() {
    println!("{}", as_str(Severity::Help));
}
