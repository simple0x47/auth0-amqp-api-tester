use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum RunMode {
    Sequential,
    Parallel,
}
