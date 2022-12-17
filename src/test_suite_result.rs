use tokio::sync::mpsc::Receiver;

use crate::test_result::TestResult;

pub struct TestSuiteResult {
    name: String,
    test_count: usize,
    results: Vec<TestResult>,
    test_result_receiver: Receiver<TestResult>,
}

impl TestSuiteResult {
    pub fn new(
        name: String,
        test_count: usize,
        test_result_receiver: Receiver<TestResult>,
    ) -> TestSuiteResult {
        TestSuiteResult {
            name,
            test_count,
            results: Vec::with_capacity(test_count),
            test_result_receiver,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn results(&self) -> &[TestResult] {
        self.results.as_slice()
    }

    pub async fn collect_results(&mut self) {
        loop {
            if let Some(result) = self.test_result_receiver.recv().await {
                self.results.push(result);
            }

            if self.results.len() == self.test_count {
                break;
            }
        }
    }

    pub fn has_any_test_failed(&self) -> bool {
        self.results.iter().any(|result| result.result().is_err())
    }
}
