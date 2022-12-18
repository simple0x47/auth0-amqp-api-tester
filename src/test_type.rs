use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy)]
pub enum TestType {
    Assert,
    Stress { times: usize },
}
