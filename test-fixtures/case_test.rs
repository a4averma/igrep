/// Test file for case-sensitive vs case-insensitive search
pub struct FooBar {
    count: usize,
}

impl FooBar {
    pub fn new() -> Self {
        FooBar { count: 0 }
    }
}

pub fn test_case_sensitivity() {
    let foo_bar = FooBar::new();
    let foobar = 42;
    let FOOBAR = 99;
    println!(
        "foo_bar: {:?}, foobar: {}, FOOBAR: {}",
        foo_bar.count, foobar, FOOBAR
    );
}
