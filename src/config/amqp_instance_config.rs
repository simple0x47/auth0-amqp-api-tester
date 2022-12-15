use lapin::{
    options::{BasicConsumeOptions, BasicPublishOptions},
    types::FieldTable,
};

use crate::error::{Error, ErrorKind};

use super::amqp::Amqp;

#[derive(Clone)]
pub struct AmqpInstanceConfig {
    publish_options: BasicPublishOptions,
    publish_arguments: FieldTable,
    consume_options: BasicConsumeOptions,
    consume_arguments: FieldTable,
}

impl AmqpInstanceConfig {
    pub fn new(
        publish_options: BasicPublishOptions,
        publish_arguments: FieldTable,
        consume_options: BasicConsumeOptions,
        consume_arguments: FieldTable,
    ) -> AmqpInstanceConfig {
        AmqpInstanceConfig {
            publish_options,
            publish_arguments,
            consume_options,
            consume_arguments,
        }
    }

    pub fn publish_options(&self) -> &BasicPublishOptions {
        &self.publish_options
    }

    pub fn publish_arguments(&self) -> &FieldTable {
        &self.publish_arguments
    }

    pub fn consume_options(&self) -> &BasicConsumeOptions {
        &self.consume_options
    }

    pub fn consume_arguments(&self) -> &FieldTable {
        &self.consume_arguments
    }
}

pub fn try_get_from_request_and_reply_amqp(
    request_amqp: &Amqp,
    reply_amqp: &Amqp,
) -> Result<AmqpInstanceConfig, Error> {
    let publish_options = match request_amqp.publish_options() {
        Some(publish_options) => publish_options.clone(),
        None => {
            return Err(Error::new(
                ErrorKind::InternalFailure,
                "request amqp does not contain publish options",
            ))
        }
    };

    let publish_arguments = match request_amqp.publish_arguments() {
        Some(publish_arguments) => publish_arguments.clone(),
        None => {
            return Err(Error::new(
                ErrorKind::InternalFailure,
                "request amqp does not contain publish arguments",
            ))
        }
    };

    let consume_options = match reply_amqp.consume_options() {
        Some(consume_options) => consume_options.clone(),
        None => {
            return Err(Error::new(
                ErrorKind::InternalFailure,
                "reply amqp does not contain consume options",
            ))
        }
    };

    let consume_arguments = match reply_amqp.consume_arguments() {
        Some(consume_arguments) => consume_arguments.clone(),
        None => {
            return Err(Error::new(
                ErrorKind::InternalFailure,
                "reply amqp does not contain consume arguments",
            ))
        }
    };

    Ok(AmqpInstanceConfig::new(
        publish_options,
        publish_arguments,
        consume_options,
        consume_arguments,
    ))
}
