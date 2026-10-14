// What if `Prepared` had no lifetime parameter?
trait Backend {
    type Prepared;

    fn prepare(source: &str) -> Self::Prepared;
}

struct TreeWalker;

impl Backend for TreeWalker {
    // The tree-walker wants to keep borrowing the program it was given.
    type Prepared = &str;

    fn prepare(source: &str) -> &str {
        source
    }
}

fn main() {}
