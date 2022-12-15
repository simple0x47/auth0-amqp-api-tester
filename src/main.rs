use crate::amqp_connection_manager::AmqpConnectionManager;
use crate::test_runner::TestRunner;
use std::io::{Error, ErrorKind};

mod amqp_connection_manager;
mod config;
mod error;
mod test;
mod test_reader;
mod test_request;
mod test_result;
mod test_run_instance;
mod test_run_mode;
mod test_runner;
mod test_type;
mod token_retriever;

#[tokio::main]
async fn main() -> Result<(), Error> {
    match simple_logger::init() {
        Ok(_) => (),
        Err(error) => {
            return Err(Error::new(
                ErrorKind::Other,
                format!("failed to initialize logger: {}", error),
            ));
        }
    }

    let arguments: Vec<String> = std::env::args().collect();

    if arguments.len() < 3 {
        return Err(Error::new(ErrorKind::InvalidInput, "no test file provided"));
    }

    let token_request_uri = match arguments.get(1) {
        Some(token_request_uri) => token_request_uri.to_string(),
        None => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "no token request uri provided",
            ));
        }
    };

    let token_request_body = match arguments.get(2) {
        Some(token_request_body) => token_request_body.to_string(),
        None => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "no token request body provided",
            ));
        }
    };

    let token = match token_retriever::try_get_token(token_request_uri, token_request_body).await {
        Ok(token) => token,
        Err(error) => {
            return Err(Error::new(
                ErrorKind::Other,
                format!("failed to get token: {}", error),
            ));
        }
    };

    log::info!("obtained token correctly!");

    let tests = match test_reader::read(&arguments[3..], token.as_str()).await {
        Ok(tests) => tests,
        Err(error) => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("failed to read test files: {}", error),
            ));
        }
    };

    let tests_lenght = tests.len();

    let amqp_connection_manager_config =
        match config::amqp_connection_manager_config::try_generate_config() {
            Ok(amqp_connection_manager_config) => amqp_connection_manager_config,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!(
                        "failed to generate amqp connection manager config: {}",
                        error
                    ),
                ));
            }
        };

    let amqp_connection_manager =
        match AmqpConnectionManager::try_new(amqp_connection_manager_config).await {
            Ok(amqp_connection_manager) => amqp_connection_manager,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("failed to create amqp connection manager: {}", error),
                ));
            }
        };

    let (result_sender, mut result_receiver) = tokio::sync::mpsc::channel(1024);

    tokio::spawn(async move {
        for test in tests {
            let amqp_channel = match amqp_connection_manager.try_get_channel().await {
                Ok(amqp_channel) => amqp_channel,
                Err(error) => {
                    match result_sender
                        .send(Err(crate::error::Error::new(
                            crate::error::ErrorKind::InternalFailure,
                            format!("failed to get amqp channel: {}", error),
                        )))
                        .await
                    {
                        Ok(_) => (),
                        Err(error) => {
                            log::error!(
                                "failed to send internal failure to main thread: {}",
                                error
                            );
                            std::process::exit(1);
                        }
                    }

                    break;
                }
            };

            let test_runner = TestRunner::new(amqp_channel, result_sender.clone());

            tokio::spawn(async move {
                test_runner.run(test).await;
            });
        }
    });

    let mut exit_code = 0;
    let mut test_count = 0;

    loop {
        let message = match result_receiver.recv().await {
            Some(message) => match message {
                Ok(_) => {
                    test_count += 1;
                    log::info!("test passed");
                }
                Err(error) => {
                    test_count += 1;
                    log::warn!("test failed: {}", error);
                    exit_code = 1;
                }
            },
            None => (),
        };

        if test_count >= tests_lenght {
            break;
        }
    }

    std::process::exit(exit_code);
}
