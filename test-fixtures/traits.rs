/// Trait definitions for the search engine
pub trait Searchable {
    fn search(&self, query: &str) -> Vec<String>;
    fn search_regex(&self, pattern: &str) -> Vec<String>;
}

pub trait Indexable {
    fn index(&mut self) -> Result<(), String>;
    fn reindex(&mut self) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub path: String,
    pub trigrams: Vec<[u8; 3]>,
    pub size: u64,
}

impl IndexEntry {
    pub fn new(path: String) -> Self {
        IndexEntry {
            path,
            trigrams: Vec::new(),
            size: 0,
        }
    }
}

// FIXME: implement Searchable for IndexEntry
pub fn compute_trigrams(data: &[u8]) -> Vec<[u8; 3]> {
    if data.len() < 3 {
        return Vec::new();
    }
    data.windows(3).map(|w| [w[0], w[1], w[2]]).collect()
}
