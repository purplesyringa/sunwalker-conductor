use crate::problem::program;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct StrategyFactory {
    pub files: HashMap<String, FileType>,
    pub blocks: Vec<Block>,
    pub programs: HashMap<String, program::CachedProgram>,
    pub root: PathBuf,
}

#[derive(Serialize)]
pub struct Block {
    pub name: String,
    pub tactic: Tactic,
    pub bindings: HashMap<String, Binding>,
    pub command: String,
    pub argv: Vec<Pattern>,
    pub stdin: Option<Pattern>,
    pub stdout: Option<Pattern>,
    pub stderr: Option<Pattern>,
}

#[derive(Serialize)]
pub enum Tactic {
    User,
    Testlib,
}

#[derive(Serialize)]
pub enum FileType {
    Regular,
    Pipe,
}

#[derive(Serialize)]
pub struct Binding {
    pub readable: bool,
    pub writable: bool,
    pub source: Pattern,
}

#[derive(Clone, Serialize)]
pub enum Pattern {
    File(String),
    VariableText(String),
}
