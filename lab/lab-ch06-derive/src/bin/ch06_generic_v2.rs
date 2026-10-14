use lab_macros_v2::Spanned;
use pebble_span::{SourceId, Span, Spanned};

#[derive(Spanned)]
struct Tagged<T: Clone>
where
    T: std::fmt::Debug,
{
    inner: T,
    span: Span,
}

fn main() {
    let tagged = Tagged {
        inner: "hello",
        span: Span::new(SourceId(0), 4, 9),
    };
    println!("{:?} at {:?} (inner = {})", tagged.span(), tagged.span().len(), tagged.inner);
}
