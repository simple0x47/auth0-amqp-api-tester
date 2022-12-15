use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind};

const REQUEST_HEADER: &str = "header";
const REQUEST_HEADER_TOKEN: &str = "token";

#[derive(Deserialize, Serialize)]
pub struct TestRequest {
    request: Map<String, Value>,
    expected_response: Value,
}

impl TestRequest {
    pub fn request(&self) -> &Map<String, Value> {
        &self.request
    }

    pub fn inject_token(&mut self, token: &str) -> Result<(), Error> {
        let header = match self.request.get_mut(REQUEST_HEADER) {
            Some(header) => match header.as_object_mut() {
                Some(header) => header,
                None => {
                    return Err(Error::new(
                        ErrorKind::InternalFailure,
                        format!("failed to get header as object"),
                    ));
                }
            },
            None => {
                return Err(Error::new(
                    ErrorKind::InternalFailure,
                    format!("failed to get header"),
                ));
            }
        };

        if header.contains_key(REQUEST_HEADER_TOKEN) {
            return Ok(());
        }

        header.insert(
            REQUEST_HEADER_TOKEN.to_string(),
            serde_json::Value::String(token.to_string()),
        );

        Ok(())
    }

    pub fn is_response_equal_to_expected(&self, response: Value) -> bool {
        self.expected_response == response
    }
}
