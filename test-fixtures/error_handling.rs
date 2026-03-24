use std::fs;
use std::io;

/// Custom error type for search operations
#[derive(Debug)]
pub enum SearchError {
    IoError(io::Error),
    PatternError(String),
    Timeout,
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::IoError(e) => write!(f, "IO error: {}", e),
            SearchError::PatternError(msg) => write!(f, "Pattern error: {}", msg),
            SearchError::Timeout => write!(f, "Search timed out"),
        }
    }
}

impl From<io::Error> for SearchError {
    fn from(err: io::Error) -> Self {
        SearchError::IoError(err)
    }
}

// TODO: implement std::error::Error trait
pub fn read_file_safe(path: &str) -> Result<String, SearchError> {
    let content = fs::read_to_string(path)?;
    if content.is_empty() {
        return Err(SearchError::PatternError("empty file".to_string()));
    }
    Ok(content)
}

pub struct ErrorStats {
    pub total_errors: usize,
    pub io_errors: usize,
    pub pattern_errors: usize,
}
