use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub enum TestVerdict {
    InQueue,
    Running,
    Ignored,

    Accepted,
    PartialSolution(u64), // in 10000 increments

    Bug(String),

    WrongAnswer,
    RuntimeError(ExitStatus),
    TimeLimitExceeded,
    MemoryLimitExceeded,
    PresentationError,
    IdlenessLimitExceeded,
    CheckerFailed,
}

#[derive(Debug, Deserialize)]
pub struct TestJudgementResult {
    pub verdict: TestVerdict,
    pub logs: HashMap<String, Vec<u8>>,
    pub invocation_stats: HashMap<String, InvocationStat>,
}

#[derive(Debug, Deserialize)]
pub struct InvocationStat {
    pub real_time: std::time::Duration,
    pub cpu_time: std::time::Duration,
    pub user_time: std::time::Duration,
    pub sys_time: std::time::Duration,
    pub memory: usize,
}

#[derive(Debug, Serialize)]
pub struct InvocationLimit {
    pub real_time: std::time::Duration,
    pub cpu_time: std::time::Duration,
    pub memory: usize,
}

#[derive(Debug, Deserialize)]
pub enum ExitStatus {
    ExitCode(u8),
    Signal(u8),
}
