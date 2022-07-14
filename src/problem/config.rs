use crate::problem::strategy;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct ProblemRevision {
    pub dependency_graph: DependencyGraph,
    pub strategy_factory: strategy::StrategyFactory,
}

#[derive(Serialize)]
pub struct DependencyGraph {
    pub dependents_of: HashMap<u64, Vec<u64>>,
}
