use pebble_nostd::bump::BumpAllocator;
use pebble_span::SourceMap;

#[global_allocator]
static HEAP: BumpAllocator<{ 1024 * 1024 }> = BumpAllocator::new();

fn main() {
    let src = "(1 + 2) * 3 - 4 / 2";

    let t0 = HEAP.bytes_used();
    let mut sources = SourceMap::new();
    let id = sources.add("<nostd>", src);
    let t1 = HEAP.bytes_used();
    let (tokens, diagnostics) = pebble_lexer::tokenize(id, src);
    let t2 = HEAP.bytes_used();
    println!("SourceMap::add     {:>5} bytes", t1 - t0);
    println!("tokenize           {:>5} bytes ({} tokens, {} diagnostics)", t2 - t1, tokens.len(), diagnostics.len());
    println!("size_of::<Token>() = {}", std::mem::size_of::<pebble_lexer::Token<'_>>());
    let t3 = HEAP.bytes_used();
    let result = pebble_nostd::eval::eval_source(src);
    let t4 = HEAP.bytes_used();
    println!("eval_source (all)  {:>5} bytes -> {result:?}", t4 - t3);
}
