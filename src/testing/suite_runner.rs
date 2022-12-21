use std::pin::Pin;
use std::sync::Arc;

use crate::amqp_connection_manager::AmqpConnectionManager;
use crate::config::amqp_instance_config::{self};
use crate::error::{Error, ErrorKind};
use futures_util::stream::FuturesUnordered;
use futures_util::Future;
use lapin::options::QueueDeleteOptions;
use lapin::{Channel, Queue};
use tokio::sync::mpsc::Sender;
use crate::testing::assert_script_runner::AssertScriptRunner;
use crate::testing::test_result::TestResult;
use crate::testing::run_instance::RunInstance;
use crate::testing::run_mode::RunMode;
use crate::testing::suite::Suite;
use crate::testing::suite_result::SuiteResult;
use crate::testing::test_type::TestType;

/// Executes test suites appropriately depending on their run mode and test type.
pub struct SuiteRunner {
    amqp_connection_manager: Arc<AmqpConnectionManager>,
    test_suite_result_sender: Sender<SuiteResult>,
    test_tasks: FuturesUnordered<Pin<Box<dyn Future<Output = ()> + Send + Sync>>>,
    stress_mode: bool,
}

impl SuiteRunner {
    pub fn new(
        amqp_connection_manager: Arc<AmqpConnectionManager>,
        test_suite_result_sender: Sender<SuiteResult>,
    ) -> SuiteRunner {
        SuiteRunner {
            amqp_connection_manager,
            test_suite_result_sender,
            test_tasks: FuturesUnordered::new(),
            stress_mode: false,
        }
    }

    /// Executes the given test suite and then proceeds to send a SuiteResult through the result sender.
    /// Error is returned in case of runtime errors instead of test related ones.
    pub async fn execute(&mut self, mut test_suite: Suite) -> Result<(), Error> {
        let channel = self.amqp_connection_manager.try_get_channel().await?;

        let request_queue = self.initialize_request_queue(&test_suite, &channel).await?;
        let reply_queue = self.initialize_reply_queue(&test_suite, &channel).await?;

        let test_type = test_suite.test_type();

        let (result_sender, result_receiver) = tokio::sync::mpsc::channel(4096);

        match test_type {
            TestType::Assert => {
                self.run(
                    &mut test_suite,
                    &request_queue,
                    &reply_queue,
                    &result_sender,
                )
                .await?
            }
            TestType::Stress { times } => {
                self.stress_mode = true;

                for time in 0..times {
                    match self
                        .run(
                            &mut test_suite,
                            &request_queue,
                            &reply_queue,
                            &result_sender,
                        )
                        .await
                    {
                        Ok(_) => log::info!("run finished successfully #{}", time),
                        Err(error) => log::error!("run failed #{} : {}", time, error),
                    }
                }

                for future in self.test_tasks.iter_mut() {
                    future.await;
                }
            }
        }

        let mut test_suite_result = SuiteResult::new(
            test_suite.name().to_string(),
            test_suite.test_count(),
            result_receiver,
        );

        test_suite_result.collect_results().await;

        match channel
            .queue_delete(reply_queue.name().as_str(), QueueDeleteOptions::default())
            .await
        {
            Ok(_) => (),
            Err(error) => log::error!("failed to delete reply queue: {}", error),
        }

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
        test: &Suite,
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
        test: &Suite,
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

    async fn run(
        &mut self,
        test_suite: &mut Suite,
        request_queue: &Queue,
        reply_queue: &Queue,
        result_sender: &Sender<TestResult>,
    ) -> Result<(), Error> {
        let mode = test_suite.run_mode();

        match mode {
            RunMode::Sequential => {
                self.run_sequentially(test_suite, &request_queue, &reply_queue, &result_sender)
                    .await?;
            }
            RunMode::Parallel => {
                self.run_parallelly(test_suite, &request_queue, &reply_queue, &result_sender)
                    .await?;
            }
        }

        Ok(())
    }

    async fn run_sequentially(
        &self,
        test_suite: &mut Suite,
        request_queue: &Queue,
        reply_queue: &Queue,
        result_sender: &Sender<TestResult>,
    ) -> Result<(), Error> {
        let amqp_instance_config = amqp_instance_config::try_get_from_request_and_reply_amqp(
            test_suite.request_amqp_configuration(),
            test_suite.reply_amqp_configuration(),
        )?;
        let test_suite_name = Arc::new(test_suite.name().to_string());
        let tests = test_suite.shared_tests();

        let channel = self.amqp_connection_manager.try_get_channel().await?;

        let assert_script_runner = Arc::new(AssertScriptRunner::try_new(test_suite_name)?);

        for test in tests {
            let test_run_instance = RunInstance::new(
                test.clone(),
                channel.clone(),
                request_queue.name().to_string(),
                reply_queue.name().to_string(),
                amqp_instance_config.clone(),
                result_sender.clone(),
                assert_script_runner.clone()
            );

            test_run_instance.run().await?;
        }

        Ok(())
    }

    async fn run_parallelly(
        &mut self,
        test_suite: &mut Suite,
        request_queue: &Queue,
        reply_queue: &Queue,
        result_sender: &Sender<TestResult>,
    ) -> Result<(), Error> {
        let amqp_instance_config = amqp_instance_config::try_get_from_request_and_reply_amqp(
            test_suite.request_amqp_configuration(),
            test_suite.reply_amqp_configuration(),
        )?;
        let test_suite_name = Arc::new(test_suite.name().to_string());
        let tests = test_suite.shared_tests();

        let assert_script_runner = Arc::new(AssertScriptRunner::try_new(test_suite_name.clone())?);

        for test in tests {
            let test_suite_name_clone = test_suite_name.clone();
            let test_name = test.name().to_string();
            let channel = self.amqp_connection_manager.try_get_channel().await?;

            let test_run_instance = RunInstance::new(
                test.clone(),
                channel,
                request_queue.name().to_string(),
                reply_queue.name().to_string(),
                amqp_instance_config.clone(),
                result_sender.clone(),
                assert_script_runner.clone()
            );

            let instance_execution = async move {
                match test_run_instance.run().await {
                    Ok(_) => log::info!("test '{}' run instance finished", test_name),
                    Err(error) => {
                        log::error!(
                            "[{}] test '{}' run instance failed: {}",
                            test_suite_name_clone,
                            test_name,
                            error
                        );
                    }
                }
            };

            if self.stress_mode {
                self.test_tasks.push(Box::pin(instance_execution));
            } else {
                tokio::spawn(instance_execution);
            }
        }

        Ok(())
    }
}
