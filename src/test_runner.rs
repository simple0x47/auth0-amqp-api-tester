use crate::amqp_connection_manager::{self, AmqpConnectionManager};
use crate::config::amqp_instance_config::{self, AmqpInstanceConfig};
use crate::error::{Error, ErrorKind};
use crate::test::Test;
use crate::test_request::TestRequest;
use crate::test_run_instance::TestRunInstance;
use crate::test_run_mode::TestRunMode;
use futures_util::TryStreamExt;
use lapin::options::{BasicAckOptions, BasicPublishOptions};
use lapin::types::ShortString;
use lapin::{BasicProperties, Channel, Consumer, Queue};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub struct TestRunner {
    channel: Channel,
    result_sender: Sender<Result<(), Error>>,
}

impl TestRunner {
    pub fn new(channel: Channel, result_sender: Sender<Result<(), Error>>) -> TestRunner {
        TestRunner {
            channel,
            result_sender,
        }
    }

    pub async fn run(&self, test: Test) -> Result<(), Error> {
        let request_queue = self.initialize_request_queue(&test, &self.channel).await?;
        let reply_queue = self.initialize_reply_queue(&test, &self.channel).await?;

        let mode = test.run_mode();

        match mode {
            TestRunMode::Sequential => {
                self.run_sequentially(test, &request_queue, &reply_queue)
                    .await?;
            }
            TestRunMode::Parallel => {
                self.run_parallelly(test, &request_queue, &reply_queue)
                    .await?;
            }
        }

        Ok(())
    }

    async fn initialize_request_queue(
        &self,
        test: &Test,
        channel: &Channel,
    ) -> Result<Queue, Error> {
        let request_queue_config = test.request_amqp_configuration().queue();

        let request_queue = match channel
            .queue_declare(
                request_queue_config.name(),
                request_queue_config.declare_options().clone(),
                request_queue_config.declare_arguments().clone(),
            )
            .await
        {
            Ok(request_queue) => request_queue,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::InternalFailure,
                    format!("failed to declare request queue: {}", error),
                ));
            }
        };

        Ok(request_queue)
    }

    async fn initialize_reply_queue(&self, test: &Test, channel: &Channel) -> Result<Queue, Error> {
        let reply_queue_config = test.reply_amqp_configuration().queue();

        let reply_queue = match channel
            .queue_declare(
                reply_queue_config.name(),
                reply_queue_config.declare_options().clone(),
                reply_queue_config.declare_arguments().clone(),
            )
            .await
        {
            Ok(reply_queue) => reply_queue,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::InternalFailure,
                    format!("failed to declare response queue: {}", error),
                ));
            }
        };

        Ok(reply_queue)
    }

    async fn run_sequentially(
        &self,
        test: Test,
        request_queue: &Queue,
        reply_queue: &Queue,
    ) -> Result<(), Error> {
        let amqp_instance_config = amqp_instance_config::try_get_from_request_and_reply_amqp(
            test.request_amqp_configuration(),
            test.reply_amqp_configuration(),
        )?;
        let test_requests = test.owned_requests();

        for test_request in test_requests {
            let test_run_instance = TestRunInstance::new(
                test_request,
                self.channel.clone(),
                request_queue.name().to_string(),
                reply_queue.name().to_string(),
                amqp_instance_config.clone(),
                self.result_sender.clone(),
            );

            test_run_instance.run().await?;
        }

        Ok(())
    }

    async fn run_parallelly(
        &self,
        test: Test,
        request_queue: &Queue,
        reply_queue: &Queue,
    ) -> Result<(), Error> {
        let amqp_instance_config = amqp_instance_config::try_get_from_request_and_reply_amqp(
            test.request_amqp_configuration(),
            test.reply_amqp_configuration(),
        )?;
        let test_requests = test.owned_requests();

        for test_request in test_requests {
            let test_run_instance = TestRunInstance::new(
                test_request,
                self.channel.clone(),
                request_queue.name().to_string(),
                reply_queue.name().to_string(),
                amqp_instance_config.clone(),
                self.result_sender.clone(),
            );

            tokio::spawn(async move {
                match test_run_instance.run().await {
                    Ok(_) => (),
                    Err(error) => {
                        log::info!("test run instance failed: {}", error);
                    }
                }
            });
        }

        Ok(())
    }
}
