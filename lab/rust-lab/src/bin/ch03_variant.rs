#[derive(Debug)]
enum Shape {
    Point,
    Circle(f64),
    Rect { w: f64, h: f64 },
}

fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Point => 0.0,
        Shape::Circle(r) => 3.0 * r * r,
        Shape::Rect { w, h } => w * h,
    }
}

fn main() {
    let shapes = [Shape::Point, Shape::Circle(2.0), Shape::Rect { w: 2.0, h: 5.0 }];
    for shape in &shapes {
        println!("{shape:?} has area {}", area(shape));
    }
    println!("size_of::<Shape>() = {}", std::mem::size_of::<Shape>());
    println!("size_of::<Option<String>>() = {}", std::mem::size_of::<Option<String>>());
    println!("size_of::<String>() = {}", std::mem::size_of::<String>());
}
