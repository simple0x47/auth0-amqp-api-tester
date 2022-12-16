use std::sync::Arc;

use crate::amqp_connection_manager::{self, AmqpConnectionManager};
use crate::config::amqp_instance_config::{self, AmqpInstanceConfig};
use crate::error::{Error, ErrorKind};
use crate::test::Test;
use crate::test_result::TestResult;
use crate::test_run_instance::TestRunInstance;
use crate::test_run_mode::TestRunMode;
use crate::test_suite::TestSuite;
use crate::test_suite_result::TestSuiteResult;
use futures_util::TryStreamExt;
use lapin::options::{BasicAckOptions, BasicPublishOptions};
use lapin::types::ShortString;
use lapin::{BasicProperties, Channel, Consumer, Queue};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub struct TestSuiteRunner {
    channel: Channel,
    test_suite_result_sender: Sender<TestSuiteResult>,
}

impl TestSuiteRunner {
    pub fn new(
        channel: Channel,
        test_suite_result_sender: Sender<TestSuiteResult>,
    ) -> TestSuiteRunner {
        TestSuiteRunner {
            channel,
            test_suite_result_sender,
        }
    }

    pub async fn run(&self, test_suite: TestSuite) -> Result<(), Error> {
        let request_queue = self
            .initialize_request_queue(&test_suite, &self.channel)
            .await?;
        let reply_queue = self
            .initialize_reply_queue(&test_suite, &self.channel)
            .await?;

        let mode = test_suite.run_mode();

        let (result_sender, result_receiver) = tokio::sync::mpsc::channel(1024);
        let mut test_suite_result = TestSuiteResult::new(
            test_suite.name().to_string(),
            test_suite.test_count(),
            result_receiver,
        );

        match mode {
            TestRunMode::Sequential => {
                self.run_sequentially(test_suite, &request_queue, &reply_queue, &result_sender)
                    .await?;
            }
            TestRunMode::Parallel => {
                self.run_parallelly(test_suite, &request_queue, &reply_queue, &result_sender)
                    .await?;
            }
        }

        test_suite_result.collect_results().await;

        match self.test_suite_result_sender.send(test_suite_result).await {
            Ok(_) => {}
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::InternalFailure,
                    format!("failed to send test suite result: {}", error),
                ));
            }
        }

        Ok(())
    }

    async fn initialize_request_queue(
        &self,
        test: &TestSuite,
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

    async fn initialize_reply_queue(
        &self,
        test: &TestSuite,
        channel: &Channel,
    ) -> Result<Queue, Error> {
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
        test_suite: TestSuite,
        request_queue: &Queue,
        reply_queue: &Queue,
        result_sender: &Sender<TestResult>,
    ) -> Result<(), Error> {
        let amqp_instance_config = amqp_instance_config::try_get_from_request_and_reply_amqp(
            test_suite.request_amqp_configuration(),
            test_suite.reply_amqp_configuration(),
        )?;
        let test_suite_name = test_suite.name().to_string();
        let tests = test_suite.owned_tests();

        log::info!("# running test suite '{}' sequentially #", test_suite_name);

        for test in tests {
            let test_run_instance = TestRunInstance::new(
                test,
                self.channel.clone(),
                request_queue.name().to_string(),
                reply_queue.name().to_string(),
                amqp_instance_config.clone(),
                result_sender.clone(),
            );

            test_run_instance.run().await?;
        }

        Ok(())
    }

    async fn run_parallelly(
        &self,
        test_suite: TestSuite,
        request_queue: &Queue,
        reply_queue: &Queue,
        result_sender: &Sender<TestResult>,
    ) -> Result<(), Error> {
        let amqp_instance_config = amqp_instance_config::try_get_from_request_and_reply_amqp(
            test_suite.request_amqp_configuration(),
            test_suite.reply_amqp_configuration(),
        )?;
        let test_suite_name = Arc::new(test_suite.name().to_string());
        let tests = test_suite.owned_tests();

        log::info!("# running test suite '{}' parallelly #", test_suite_name);

        let mut test_tasks = Vec::new();

        for test in tests {
            let test_suite_name_clone = test_suite_name.clone();
            let test_name = test.name().to_string();

            let test_run_instance = TestRunInstance::new(
                test,
                self.channel.clone(),
                request_queue.name().to_string(),
                reply_queue.name().to_string(),
                amqp_instance_config.clone(),
                result_sender.clone(),
            );

            test_tasks.push(tokio::spawn(async move {
                match test_run_instance.run().await {
                    Ok(_) => (),
                    Err(error) => {
                        log::error!(
                            "[{}] test '{}' run instance failed: {}",
                            test_suite_name_clone,
                            test_name,
                            error
                        );
                    }
                }
            }));
        }

        for test_task in test_tasks {
            match test_task.await {
                Ok(_) => (),
                Err(error) => {
                    return Err(Error::new(
                        ErrorKind::InternalFailure,
                        format!("test task failed: {}", error),
                    ));
                }
            }
        }

        Ok(())
    }
}
