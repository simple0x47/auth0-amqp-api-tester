use crate::error::ErrorKind;
use crate::test_request::TestRequest;
use crate::test_run_mode::TestRunMode;
use crate::test_type::TestType;
use crate::{config::amqp::Amqp, error::Error};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Deserialize, Serialize)]
pub struct Test {
    name: String,
    test_type: TestType,
    run_mode: TestRunMode,
    requests: Vec<TestRequest>,
    request_amqp_configuration: Amqp,
    reply_amqp_configuration: Amqp,
}

impl<'a> Test {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn test_type(&self) -> &TestType {
        &self.test_type
    }

    pub fn run_mode(&self) -> &TestRunMode {
        &self.run_mode
    }

    pub fn requests(&self) -> &Vec<TestRequest> {
        &self.requests
    }

    pub fn mut_requests(&mut self) -> &mut Vec<TestRequest> {
        &mut self.requests
    }

    pub fn owned_requests(self) -> Vec<TestRequest> {
        self.requests
    }

    pub fn request_amqp_configuration(&self) -> &Amqp {
        &self.request_amqp_configuration
    }

    pub fn reply_amqp_configuration(&self) -> &Amqp {
        &self.reply_amqp_configuration
    }
}
