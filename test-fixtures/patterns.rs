use std::fmt;

#[derive(Debug)]
pub struct MyStruct {
    name: String,
    value: i32,
}

pub fn example() -> MyStruct {
    MyStruct {
        name: String::from("test"),
        value: 42,
    }
}

impl fmt::Display for MyStruct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MyStruct({}, {})", self.name, self.value)
    }
}

pub enum Color {
    Red,
    Green,
    Blue,
}

// FIXME: add more colors
