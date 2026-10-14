#[derive(Debug, Clone)]
struct Config {
    name: String,
    retries: u32,
    verbose: bool,
}

impl Config {
    fn new(name: &str) -> Self {
        Self { name: name.to_string(), retries: 3, verbose: false }
    }
    fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }
    fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

fn describe(label: Option<&str>) -> String {
    let Some(label) = label else {
        return "no label".to_string();
    };
    if let Some(first) = label.chars().next() {
        format!("label {label} starts with {first}")
    } else {
        "empty label".to_string()
    }
}

fn main() {
    let base = Config::new("demo").retries(5).verbose(true);
    let quiet = Config { verbose: false, ..base.clone() };
    println!("{base:?}");
    println!("{quiet:?}");
    println!("{}", describe(None));
    println!("{}", describe(Some("")));
    println!("{}", describe(Some("x")));
}
