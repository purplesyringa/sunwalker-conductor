use anyhow::Result;
use std::collections::HashMap;

pub struct ArchiveStore {}

pub struct BlobHandle {}

pub struct Archive {
    files: HashMap<String, ArchiveFile>,
}

pub struct ArchiveFile {
    handle: BlobHandle,
    executable: bool,
}

impl ArchiveStore {
    pub fn store_blob(&self, data: Vec<u8>) -> Result<BlobHandle> {
        unimplemented!()
    }
}

impl Archive {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, name: String, blob: BlobHandle, executable: bool) {
        self.files.insert(
            name,
            ArchiveFile {
                handle: blob,
                executable,
            },
        );
    }
}
