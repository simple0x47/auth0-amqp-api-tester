use lapin::options::{BasicConsumeOptions, BasicPublishOptions};
use lapin::types::FieldTable;
use serde::{Deserialize, Serialize};
use crate::config::amqp_queue::AmqpQueue;

#[derive(Deserialize, Serialize)]
pub struct Amqp {
    queue: AmqpQueue,
    publish_options: Option<BasicPublishOptions>,
    publish_arguments: Option<FieldTable>,
    consume_options: Option<BasicConsumeOptions>,
    consume_arguments: Option<FieldTable>
}

impl Amqp {
    pub fn queue(&self) -> &AmqpQueue {
        &self.queue
    }

    pub fn publish_options(&self) -> &Option<BasicPublishOptions> {
        &self.publish_options
    }

    pub fn publish_arguments(&self) -> &Option<FieldTable> {
        &self.publish_arguments
    }

    pub fn consume_options(&self) -> &Option<BasicConsumeOptions> {
        &self.consume_options
    }

    pub fn consume_arguments(&self) -> &Option<FieldTable> {
        &self.consume_arguments
    }
}