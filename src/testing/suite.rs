use std::sync::Arc;

use crate::{config::amqp::Amqp};
use serde::{Deserialize, Serialize};
use crate::testing::test::Test;
use crate::testing::run_mode::RunMode;
use crate::testing::test_type::TestType;

#[derive(Deserialize, Serialize)]
pub struct Suite {
    name: String,
    test_type: TestType,
    run_mode: RunMode,
    tests: Vec<Test>,
    request_amqp_configuration: Amqp,
    reply_amqp_configuration: Amqp,

    #[serde(skip)]
    shared_tests: Vec<Arc<Test>>,
}

impl<'a> Suite {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn test_type(&self) -> TestType {
        self.test_type
    }

    pub fn run_mode(&self) -> &RunMode {
        &self.run_mode
    }

    pub fn test_count(&self) -> usize {
        match self.test_type {
            TestType::Assert => self.tests.len(),
            TestType::Stress { times } => self.tests.len() * times,
        }
    }

    pub fn mut_tests(&mut self) -> &mut Vec<Test> {
        &mut self.tests
    }

    pub fn shared_tests(&mut self) -> &[Arc<Test>] {
        if self.shared_tests.is_empty() {
            for test in &self.tests {
                let shared_test = Arc::new(test.clone());
                self.shared_tests.push(shared_test);
            }
        }

        self.shared_tests.as_slice()
    }

    pub fn request_amqp_configuration(&self) -> &Amqp {
        &self.request_amqp_configuration
    }

    pub fn reply_amqp_configuration(&self) -> &Amqp {
        &self.reply_amqp_configuration
    }
}
