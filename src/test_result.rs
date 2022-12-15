use crate::error::Error;

pub struct TestResult {
    id: String,
    result: Result<(), Error>,
}

impl TestResult {
    pub fn new(id: String, result: Result<(), Error>) -> TestResult {
        TestResult { id, result }
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    pub fn result(&self) -> &Result<(), Error> {
        &self.result
    }
}
