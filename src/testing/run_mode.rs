use serde::{Deserialize, Serialize};

/// Modes for running test suites.
#[derive(Serialize, Deserialize)]
pub enum RunMode {
    /// Each test is run sequentially in a single task.
    Sequential,
    /// Each test has its own execution wrapped into a task.
    Parallel,
}
