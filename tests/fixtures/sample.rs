fn greet(name: &str) -> String {
    format!("hello, {name}")
}

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn origin() -> Self {
        Self { x: 0, y: 0 }
    }
}
