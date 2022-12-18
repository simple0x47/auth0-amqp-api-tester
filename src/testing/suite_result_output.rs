use crate::error::Error;
use crate::testing::suite_result::SuiteResult;

/// Outputs the test suite results into an useful format.
pub fn output(suite_result: SuiteResult) -> Result<(), Error> {
    log::info!("# test suite '{}' results #", suite_result.name());
    let test_results = suite_result.results();

    for test_result in test_results {
        match test_result.result() {
            Ok(()) => log::info!("OK   - test '{}'", test_result.id()),
            Err(error) => log::info!("FAIL - test '{}' : {}", test_result.id(), error),
        }
    }

    Ok(())
}
