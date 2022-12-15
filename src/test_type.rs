use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub enum TestType {
    Single,
    Stress { tasks: u32, requests: u32 }
}