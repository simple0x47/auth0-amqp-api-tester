use crate::{error::Error, test_suite_result::TestSuiteResult};

pub fn output(test_suite_result: TestSuiteResult) -> Result<(), Error> {
    log::info!("# test suite '{}' results #", test_suite_result.name());
    let test_results = test_suite_result.results();

    for test_result in test_results {
        match test_result.result() {
            Ok(()) => log::info!("OK   - test '{}'", test_result.id()),
            Err(error) => log::info!("FAIL - test '{}' : {}", test_result.id(), error),
        }
    }

    Ok(())
}
