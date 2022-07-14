use crate::verdict::InvocationLimit;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub enum Message {
    AddSubmission(AddSubmission),
    PushToJudgementQueue(PushToJudgementQueue),
    CancelJudgementOnTests(CancelJudgementOnTests),
    FinalizeSubmission(FinalizeSubmission),
    SupplyFile(SupplyFile),
}

#[derive(Debug, Serialize)]
pub struct AddSubmission {
    pub compilation_core: u64,
    pub submission_id: String,
    pub problem_id: String,
    pub revision_id: String,
    pub files: HashMap<String, Vec<u8>>,
    pub language: String,
    pub invocation_limits: HashMap<String, InvocationLimit>,
}

#[derive(Debug, Serialize)]
pub struct PushToJudgementQueue {
    pub core: u64,
    pub submission_id: String,
    pub tests: Vec<u64>,
}

#[derive(Debug, Serialize)]
pub struct CancelJudgementOnTests {
    pub submission_id: String,
    pub failed_tests: Vec<u64>,
}

#[derive(Debug, Serialize)]
pub struct FinalizeSubmission {
    pub submission_id: String,
}

#[derive(Debug, Serialize)]
pub struct SupplyFile {
    pub request_id: u64,
    pub contents: Vec<u8>,
}
