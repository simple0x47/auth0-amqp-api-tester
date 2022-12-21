use std::sync::Arc;
use tokio::process::Command;
use crate::error::{Error, ErrorKind};

const PYTHON_3_BIN_ENV: &str = "PYTHON_3_BIN";

const DEFAULT_INTEGRATION_TESTS_PATH: &str = "./integration_tests";

pub struct AssertScriptRunner {
    test_suite_name: Arc<String>,
    python_bin: String,
}

impl AssertScriptRunner {
    pub fn try_new(test_suite_name: Arc<String>) -> Result<AssertScriptRunner, Error> {
        let python_bin = match std::env::var(PYTHON_3_BIN_ENV) {
            Ok(python_bin) => python_bin,
            Err(error) => return Err(Error::new(ErrorKind::InternalFailure, format!("failed to read python bin environment variable: {}", error)))
        };

        Ok(AssertScriptRunner {
            test_suite_name,
            python_bin
        })
    }

    pub async fn run_script(&self, script_name: &str, response: Vec<u8>) -> Result<(), Error> {
        let response = match String::from_utf8(response) {
            Ok(response) => response,
            Err(error) => return Err(Error::new(ErrorKind::InternalFailure, format!("failed to decode response: {}", error)))
        };

        let file_path = format!("{}/{}/{}", DEFAULT_INTEGRATION_TESTS_PATH, self.test_suite_name, script_name);
        let result = match Command::new(&self.python_bin)
            .arg(&file_path)
            .arg(response)
            .spawn() {
            Ok(mut process) => process.wait().await,
            Err(error) => return Err(Error::new(ErrorKind::InternalFailure, format!("failed to spawn process: {}", error)))
        };

        match result {
            Ok(exit_status) => if exit_status.success() {
                Ok(())
            } else {
                return Err(Error::new(ErrorKind::TestAssertFailure, format!("assertion script '{}' failed: '{}'", file_path, exit_status)))
            },
            Err(error) => Err(Error::new(ErrorKind::InternalFailure,
                                         format!("failed to wait for process to end: {}", error),
            )),
        }
    }
}