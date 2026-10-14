// A miniature of the `Backend` trait from pebble-cli, without the Pebble types.
trait Backend {
    const NAME: &'static str;
    type Prepared<'p>;

    fn prepare(source: &str) -> Self::Prepared<'_>;
    fn with_output() -> Self;
    fn execute(&mut self, prepared: &Self::Prepared<'_>) -> Option<String>;
}

struct Echo;

impl Backend for Echo {
    const NAME: &'static str = "echo";
    type Prepared<'p> = &'p str;

    fn prepare(source: &str) -> &str {
        source
    }

    fn with_output() -> Self {
        Echo
    }

    fn execute(&mut self, prepared: &&str) -> Option<String> {
        Some(prepared.to_uppercase())
    }
}

fn main() {
    let backend: Box<dyn Backend> = Box::new(Echo);
    let _ = backend;
}
