// The working shape: a generic associated type, plus an associated const read
// through a type parameter.
trait Backend {
    const NAME: &'static str;
    type Prepared<'p>;

    fn prepare(source: &str) -> Self::Prepared<'_>;
    fn run(prepared: &Self::Prepared<'_>) -> usize;
}

struct Borrowing;
struct Owning;

impl Backend for Borrowing {
    const NAME: &'static str = "borrowing";
    type Prepared<'p> = &'p str;

    fn prepare(source: &str) -> &str {
        source
    }

    fn run(prepared: &&str) -> usize {
        prepared.len()
    }
}

impl Backend for Owning {
    const NAME: &'static str = "owning";
    type Prepared<'p> = Vec<char>;

    fn prepare(source: &str) -> Vec<char> {
        source.chars().collect()
    }

    fn run(prepared: &Vec<char>) -> usize {
        prepared.len()
    }
}

fn report<B: Backend>(source: &str) {
    let prepared = B::prepare(source);
    println!("{:<12} {}", format!("{}:", B::NAME), B::run(&prepared));
}

fn main() {
    let source = String::from("pebble");
    report::<Borrowing>(&source);
    report::<Owning>(&source);
}
