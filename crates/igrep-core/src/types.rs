use std::path::PathBuf;

pub type DocId = u32;
pub type NgramHash = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostingList(pub Vec<DocId>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryOp {
    All,
    None,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub op: QueryOp,
    pub trigrams: Vec<NgramHash>,
    pub children: Vec<Query>,
}

#[derive(Debug, Clone)]
pub struct IndexConfig {
    pub max_ngram_length: usize,
    pub max_file_size: u64,
    pub skip_binary: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            max_ngram_length: 16,
            max_file_size: 50 * 1024 * 1024, // 50MB
            skip_binary: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
}

#[derive(Debug, Clone)]
pub struct IndexMeta {
    pub version: u32,
    pub commit_sha: Option<String>,
    pub file_count: u32,
    pub ngram_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_config_default() {
        let config = IndexConfig::default();
        assert_eq!(config.max_ngram_length, 16);
        assert_eq!(config.max_file_size, 52_428_800);
        assert!(config.skip_binary);
    }

    #[test]
    fn test_query_construction() {
        let leaf = Query {
            op: QueryOp::And,
            trigrams: vec![1, 2, 3],
            children: vec![],
        };
        assert_eq!(leaf.op, QueryOp::And);
        assert_eq!(leaf.trigrams.len(), 3);
        assert!(leaf.children.is_empty());

        let parent = Query {
            op: QueryOp::Or,
            trigrams: vec![],
            children: vec![leaf.clone()],
        };
        assert_eq!(parent.op, QueryOp::Or);
        assert!(parent.trigrams.is_empty());
        assert_eq!(parent.children.len(), 1);

        let all = Query {
            op: QueryOp::All,
            trigrams: vec![],
            children: vec![],
        };
        assert_eq!(all.op, QueryOp::All);

        let none = Query {
            op: QueryOp::None,
            trigrams: vec![],
            children: vec![],
        };
        assert_eq!(none.op, QueryOp::None);
    }

    #[test]
    fn test_posting_list_from_vec() {
        let ids: Vec<DocId> = vec![1, 5, 10, 42];
        let pl = PostingList(ids.clone());
        assert_eq!(pl.0, ids);
        assert_eq!(pl.0.len(), 4);

        let empty = PostingList(vec![]);
        assert!(empty.0.is_empty());
    }
}
