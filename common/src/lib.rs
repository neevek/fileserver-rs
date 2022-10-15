use serde::Serialize;

#[derive(Serialize)]
pub enum FileType {
    File,
    Directory,
    SymbolicLink,
}

#[derive(Serialize)]
pub struct DirEntry {
    pub file_name: String,
    pub file_type: FileType,
    pub file_size: u64,
    pub last_accessed: String,
}

#[derive(Serialize)]
pub struct DirDesc {
    pub dir_name: String,
    pub descendants: Vec<DirEntry>,
}
