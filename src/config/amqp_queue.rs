use lapin::options::QueueDeclareOptions;
use lapin::types::FieldTable;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct AmqpQueue {
    name: String,
    declare_options: QueueDeclareOptions,
    declare_arguments: FieldTable
}

impl AmqpQueue {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn declare_options(&self) -> &QueueDeclareOptions {
        &self.declare_options
    }

    pub fn declare_arguments(&self) -> &FieldTable {
        &self.declare_arguments
    }
}