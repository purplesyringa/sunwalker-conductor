use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Problem {
    pub judging: Judging,
    pub assets: Assets,
    pub tags: Tags,
}

#[derive(Deserialize)]
pub struct Judging {
    #[serde(default)]
    pub input_file: String,

    #[serde(default)]
    pub output_file: String,

    pub run_count: Option<u64>,

    pub testset: Vec<TestSet>,
}

#[derive(Deserialize)]
pub struct TestSet {
    pub name: String,
    pub time_limit: u64,   // ms
    pub memory_limit: u64, // bytes
    pub test_count: usize,
    pub input_path_pattern: Option<String>, // C-style format string
    pub answer_path_pattern: Option<String>, // C-style format string
    pub path_pattern: Vec<PathPattern>,
    pub tests: Vec<Test>,
    pub groups: Vec<Group>,
}

#[derive(Deserialize)]
pub struct PathPattern {
    pub name: String,
    #[serde(rename = "$value")]
    pub value: String, // C-style format string
}

#[derive(Deserialize)]
pub struct Test {
    pub method: String,

    #[serde(default)]
    pub group: String,

    #[serde(default)]
    pub cmd: String,

    #[serde(default)]
    pub description: String,

    pub points: Option<f64>,

    #[serde(default)]
    pub sample: bool,
}

#[derive(Deserialize)]
pub struct Group {
    pub feedback_policy: String,
    pub name: String,
    pub points: Option<f64>,
    pub points_policy: String,
    pub dependencies: Option<Dependencies>,
}

#[derive(Deserialize)]
pub struct Dependencies {
    pub dependency: Vec<Dependency>,
}

#[derive(Deserialize)]
pub struct Dependency {
    pub group: String,
}

#[derive(Deserialize)]
pub struct Assets {
    pub checker: Checker,
    pub interactor: Option<Interactor>,
    pub strategy: Option<Strategy>,
}

#[derive(Deserialize)]
pub struct Checker {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub type_: String,
    pub source: Source,
    pub binary: Binary,
}

#[derive(Deserialize)]
pub struct Interactor {
    pub source: Source,
    pub binary: Binary,
    pub runs: Option<Runs>,
}

#[derive(Deserialize)]
pub struct Runs {
    pub run: Vec<Run>,
}

#[derive(Deserialize)]
pub struct Run {
    #[serde(rename = "$value")]
    pub value: u64,
}

#[derive(Deserialize)]
pub struct Strategy {
    pub source: Source,
}

#[derive(Deserialize)]
pub struct Source {
    pub path: String,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Deserialize)]
pub struct Binary {
    pub path: String,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Deserialize)]
pub struct Tags {
    pub tag: Vec<Tag>,
}

#[derive(Deserialize)]
pub struct Tag {
    pub value: String,
}

pub fn parse_problem_xml(problem_xml: &str) -> Result<Problem> {
    serde_xml_rs::from_str(problem_xml).context("Failed to parse problem.xml")
}
