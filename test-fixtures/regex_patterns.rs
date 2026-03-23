/// Regex pattern matching utilities
use std::collections::HashMap;

pub fn match_literal(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}

pub fn match_word_boundary(text: &str, word: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let word_chars: Vec<char> = word.chars().collect();

    for i in 0..chars.len() {
        if i + word_chars.len() <= chars.len() {
            let slice: String = chars[i..i + word_chars.len()].iter().collect();
            if slice == word {
                positions.push(i);
            }
        }
    }
    positions
}

/// Build a trigram index from text
// FIXME: handle Unicode properly in trigram extraction
pub fn build_trigram_map(text: &str) -> HashMap<String, Vec<usize>> {
    let mut map = HashMap::new();
    let bytes = text.as_bytes();

    for i in 0..bytes.len().saturating_sub(2) {
        let trigram = String::from_utf8_lossy(&bytes[i..i + 3]).to_string();
        map.entry(trigram).or_insert_with(Vec::new).push(i);
    }

    map
}

pub enum MatchType {
    Literal,
    Regex,
    Fuzzy,
}
