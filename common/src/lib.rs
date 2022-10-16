use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum FileType {
    File,
    Directory,
    SymbolicLink,
}

#[derive(Serialize, Deserialize)]
pub struct DirEntry {
    pub file_name: String,
    pub file_type: FileType,
    pub file_size: u64,
    pub last_accessed: String,
}

#[derive(Serialize, Deserialize)]
pub struct DirDesc {
    pub dir_name: String,
    pub descendants: Vec<DirEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum JsonRequest {
    CreateDirectory { dir_name: String },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum JsonResponse {
    Succeeded { msg: Option<String> },
    Failed { msg: Option<String> },
}
