trait Output {
    fn write_line(&mut self, line: &str);
}

struct Stdout;
struct Buffer(Vec<String>);

impl Output for Stdout {
    fn write_line(&mut self, line: &str) {
        println!("{line}");
    }
}

impl Output for Buffer {
    fn write_line(&mut self, line: &str) {
        self.0.push(line.to_string());
    }
}

fn greet(out: &mut dyn Output) {
    out.write_line("hello");
}

fn main() {
    let mut buffer = Buffer(Vec::new());
    greet(&mut Stdout);
    greet(&mut buffer);
    println!("captured: {:?}", buffer.0);
    println!("size_of &Buffer:     {}", std::mem::size_of::<&Buffer>());
    println!("size_of &dyn Output: {}", std::mem::size_of::<&dyn Output>());
    println!("size_of Box<dyn Output>: {}", std::mem::size_of::<Box<dyn Output>>());
}
