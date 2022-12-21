use std::sync::Arc;

use futures_util::TryStreamExt;
use lapin::{options::BasicAckOptions, BasicProperties, Channel};
use serde_json::Value;
use tokio::sync::mpsc::Sender;

use crate::{
    config::amqp_instance_config::AmqpInstanceConfig,
    error::{Error, ErrorKind},
};
use crate::testing::assert_script_runner;
use crate::testing::assert_script_runner::AssertScriptRunner;
use crate::testing::test::Test;
use crate::testing::test_result::TestResult;

/// A single test instance that is run by the SuiteRunner.
pub struct RunInstance {
    test: Arc<Test>,
    channel: Channel,
    request_queue_name: String,
    reply_queue_name: String,
    amqp_instance: AmqpInstanceConfig,
    result_sender: Sender<TestResult>,
    assert_script_runner: Arc<AssertScriptRunner>
}

impl RunInstance {
    pub fn new(
        test: Arc<Test>,
        channel: Channel,
        request_queue_name: String,
        reply_queue_name: String,
        amqp_instance: AmqpInstanceConfig,
        result_sender: Sender<TestResult>,
        assert_script_runner: Arc<AssertScriptRunner>
    ) -> Self {
        RunInstance {
            test,
            channel,
            request_queue_name,
            reply_queue_name,
            amqp_instance,
            result_sender,
            assert_script_runner
        }
    }

    pub async fn run(mut self) -> Result<(), Error> {
        let correlation_id = uuid::Uuid::new_v4().to_string();

        self = self.send_request(correlation_id.as_str()).await?;
        self.get_reply(correlation_id.as_str()).await?;

        Ok(())
    }

    async fn send_request(self, correlation_id: &str) -> Result<Self, Error> {
        log::info!("[{}] sending request with correlation_id: {}", self.test.name(), correlation_id);

        let request_payload = match serde_json::to_vec(self.test.request()) {
            Ok(request_payload) => request_payload,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::InternalFailure,
                    format!("Failed to serialize test request data: {}", error),
                ))
            }
        };

        let request_properties = BasicProperties::default()
            .with_content_type("application/json".into())
            .with_correlation_id(correlation_id.into())
            .with_reply_to(self.reply_queue_name.clone().into());

        match self
            .channel
            .basic_publish(
                "",
                self.request_queue_name.as_str(),
                *self.amqp_instance.publish_options(),
                request_payload.as_slice(),
                request_properties,
            )
            .await
        {
            Ok(_) => (),
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::InternalFailure,
                    format!("failed to publish request: {}", error),
                ))
            }
        }

        Ok(self)
    }

    async fn get_reply(self, correlation_id: &str) -> Result<Self, Error> {
        log::info!("[{}] getting reply for correlation id: {}", self.test.name(), correlation_id);

        let consumer_tag = format!("{}#{}", &self.reply_queue_name, uuid::Uuid::new_v4());

        let mut consumer = match self
            .channel
            .basic_consume(
                self.reply_queue_name.as_str(),
                consumer_tag.as_str(),
                *self.amqp_instance.consume_options(),
                self.amqp_instance.consume_arguments().clone(),
            )
            .await
        {
            Ok(consumer) => consumer,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::InternalFailure,
                    format!("failed to consume reply: {}", error),
                ))
            }
        };

        loop {
            log::info!("[{}] trying to get next delivery", self.test.name());

            let delivery = match consumer.try_next().await {
                Ok(Some(delivery)) => delivery,
                Ok(None) => {
                    return Err(Error::new(
                        ErrorKind::InternalFailure,
                        "failed to get reply: no reply received",
                    ))
                }
                Err(error) => {
                    return Err(Error::new(
                        ErrorKind::InternalFailure,
                        format!("failed to get reply: {}", error),
                    ))
                }
            };

            log::info!("[{}] received delivery", self.test.name());

            match delivery.ack(BasicAckOptions::default()).await {
                Ok(_) => (),
                Err(error) => {
                    return Err(Error::new(
                        ErrorKind::InternalFailure,
                        format!("failed to ack reply: {}", error),
                    ))
                }
            }

            if let Some(delivery_correlation_id) = delivery.properties.correlation_id() {
                if delivery_correlation_id.as_str() == correlation_id {
                    let assert_script = self.test.assert_script();

                    match self.assert_script_runner.run_script(assert_script, delivery.data).await {
                        Ok(_) => {
                            if let Err(error) = self
                                .result_sender
                                .send(TestResult::new(self.test.name().to_string(), Ok(())))
                                .await
                            {
                                return Err(Error::new(
                                    ErrorKind::InternalFailure,
                                    format!("failed to send result: {}", error),
                                ));
                            }
                        },
                        Err(assert_error) => {
                            if let Err(error) = self
                                .result_sender
                                .send(TestResult::new(
                                    self.test.name().to_string(),
                                    Err(Error::new(
                                        ErrorKind::TestAssertFailure,
                                        assert_error.message(),
                                        ),
                                    )),
                                )
                                .await
                            {
                                return Err(Error::new(
                                    ErrorKind::InternalFailure,
                                    format!("failed to send result: {}", error),
                                ));
                            }
                        }
                    }

                    break;
                }
            }
        }

        Ok(self)
    }
}
