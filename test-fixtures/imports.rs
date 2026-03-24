use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

/// Module that exercises various import patterns
pub struct ImportTest {
    map: HashMap<String, Vec<u8>>,
    tree: BTreeMap<String, usize>,
    path: PathBuf,
}

impl ImportTest {
    pub fn new() -> Self {
        ImportTest {
            map: HashMap::new(),
            tree: BTreeMap::new(),
            path: PathBuf::new(),
        }
    }

    pub fn shared(self) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(self))
    }
}

// TODO: add async imports with tokio
