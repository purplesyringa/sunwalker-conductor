use serde::Serialize;

#[derive(Serialize)]
pub struct CachedProgram {
    pub package: String,
    pub prerequisites: Vec<String>,
    pub argv: Vec<String>,
}
