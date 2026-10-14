fn main() {
    let text = "ab\n中c";
    println!("len()           = {}", text.len());
    println!("chars().count() = {}", text.chars().count());
    println!("find('c')       = {:?}", text.find('c'));
    for (offset, ch) in text.char_indices() {
        println!("  byte {offset}: {ch:?} ({} bytes)", ch.len_utf8());
    }
    let cut = std::hint::black_box(4);
    println!("is_char_boundary(4) = {}", text.is_char_boundary(cut));
    println!("{}", &text[cut..]);
}
