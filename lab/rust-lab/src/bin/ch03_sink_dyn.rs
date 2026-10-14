trait Sink {
    fn emit(&mut self, message: String);

    fn error(&mut self, message: impl Into<String>) {
        self.emit(message.into());
    }
}

impl Sink for Vec<String> {
    fn emit(&mut self, message: String) {
        self.push(message);
    }
}

fn main() {
    let mut sink: Box<dyn Sink> = Box::new(Vec::new());
    sink.emit("boom".to_string());
}
