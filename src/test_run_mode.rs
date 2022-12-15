use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum TestRunMode {
    Sequential,
    Parallel,
}
