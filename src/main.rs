use crate::amqp_connection_manager::AmqpConnectionManager;
use crate::testing::suite_result::SuiteResult;
use crate::testing::suite_runner::SuiteRunner;
use crate::testing::{suite_reader, suite_result_output};
use std::{
    io::{Error, ErrorKind},
    sync::Arc,
};

mod amqp_connection_manager;
mod config;
mod error;
mod testing;
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

    if arguments.len() != 4 {
        return Err(Error::new(ErrorKind::InvalidInput, "no test file provided"));
    }

    log::info!("# executing tester with the following arguments #");
    for argument in arguments.as_slice() {
        log::info!("\t-> {}", argument);
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

    let test_suite_files = match arguments.get(3) {
        Some(test_suite_files) => test_suite_files.split(':').collect::<Vec<&str>>(),
        None => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "no test suite files provided",
            ));
        }
    };

    let test_suites = match suite_reader::read(test_suite_files.as_slice(), token.as_str()).await {
        Ok(tests) => tests,
        Err(error) => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("failed to read test files: {}", error),
            ));
        }
    };

    let test_suites_length = test_suites.len();

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
            Ok(amqp_connection_manager) => Arc::new(amqp_connection_manager),
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("failed to create amqp connection manager: {}", error),
                ));
            }
        };

    let (result_sender, mut result_receiver) = tokio::sync::mpsc::channel::<SuiteResult>(4096);

    tokio::spawn(async move {
        for test_suite in test_suites {
            let mut test_runner =
                SuiteRunner::new(amqp_connection_manager.clone(), result_sender.clone());
            let test_name = test_suite.name().to_string();

            tokio::spawn(async move {
                match test_runner.execute(test_suite).await {
                    Ok(()) => (),
                    Err(error) => {
                        log::error!("failed to run test suite '{}': {}", test_name, error);
                        std::process::exit(1);
                    }
                }
            });
        }
    });

    let mut exit_code = 0;
    let mut test_suite_count = 0;

    loop {
        match result_receiver.recv().await {
            Some(test_suite_result) => {
                if test_suite_result.has_any_test_failed() {
                    exit_code = 1;
                }

                match suite_result_output::output(test_suite_result) {
                    Ok(()) => (),
                    Err(error) => {
                        log::error!("failed to output test suite result: {}", error);
                        std::process::exit(1);
                    }
                }
                test_suite_count += 1;
            }
            None => (),
        };

        if test_suite_count >= test_suites_length {
            break;
        }
    }

    std::process::exit(exit_code);
}
