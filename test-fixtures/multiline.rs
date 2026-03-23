/// Tests for multiline pattern matching
pub fn multiline_string() -> &'static str {
    "This is line one
and this is line two
and line three ends here"
}

pub fn contains_special_chars() -> &'static str {
    r#"Special chars: \n \t \r \0 "quotes" 'single' `backtick`"#
}

pub fn blank_lines_test() -> String {
    let mut result = String::new();
    result.push_str("before blank\n");
    result.push_str("\n");
    result.push_str("\n");
    result.push_str("after blank\n");
    result
}

// This function has a very long name for testing
pub fn this_is_a_very_long_function_name_that_should_still_be_searchable() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_multiline() {
        let text = multiline_string();
        assert!(text.contains("line one"));
        assert!(text.contains("line two"));
    }
}
