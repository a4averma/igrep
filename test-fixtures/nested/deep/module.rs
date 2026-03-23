//! A deeply nested module for testing path traversal

pub mod inner {
    pub fn deep_function() -> &'static str {
        // TODO: return something meaningful
        "deep inside nested/deep/module.rs"
    }

    pub struct DeepStruct {
        pub depth: usize,
    }
}

#[cfg(test)]
mod tests {
    use super::inner;

    fn test_deep() {
        let result = inner::deep_function();
        assert_eq!(result, "deep inside nested/deep/module.rs");
    }
}
