use crate::error::{Error, ErrorKind};
use crate::test::Test;
use crate::test_request::TestRequest;

pub async fn read(files: &[String], token: &str) -> Result<Vec<Test>, Error> {
    let mut tests = Vec::<Test>::with_capacity(files.len());

    for file in files {
        let file_content = match tokio::fs::read(format!("./{}", file)).await {
            Ok(file_content) => file_content,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::InternalFailure,
                    format!("failed to read file: {}", error),
                ));
            }
        };

        let mut test: Test = match serde_json::from_slice(file_content.as_slice()) {
            Ok(test) => test,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::InternalFailure,
                    format!("failed to deserialize test: {}", error),
                ));
            }
        };

        for request in test.mut_requests().as_mut_slice() {
            match request.inject_token(token) {
                Ok(_) => (),
                Err(error) => {
                    return Err(Error::new(
                        ErrorKind::InternalFailure,
                        format!("failed to inject token into test request: {}", error),
                    ));
                }
            }
        }

        tests.push(test);
    }

    Ok(tests)
}
