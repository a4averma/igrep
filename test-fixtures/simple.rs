use std::io;

// TODO: fix this
fn main() {
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    println!("You typed: {}", input.trim());
}
