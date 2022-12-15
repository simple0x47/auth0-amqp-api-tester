use serde_json::{Map, Value};

use crate::error::{Error, ErrorKind};

const TOKEN_RESPONSE_KEY: &str = "access_token";

pub async fn try_get_token(
    token_request_uri: String,
    token_request_body: String,
) -> Result<String, Error> {
    let client = reqwest::Client::new();
    let response = match client
        .post(token_request_uri)
        .header("content-type", "application/json")
        .body(token_request_body)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return Err(Error::new(
                ErrorKind::InternalFailure,
                format!("failed to send token request: {}", error),
            ));
        }
    };

    let response_text = match response.text().await {
        Ok(response_text) => response_text,
        Err(error) => {
            return Err(Error::new(
                ErrorKind::InternalFailure,
                format!("failed to get token response text: {}", error),
            ));
        }
    };

    let token = match serde_json::from_str::<Map<String, Value>>(response_text.as_str()) {
        Ok(parameters) => match parameters.get(TOKEN_RESPONSE_KEY) {
            Some(token) => match token.as_str() {
                Some(token) => token.to_string(),
                None => {
                    return Err(Error::new(
                        ErrorKind::InternalFailure,
                        format!("failed to deserialize token as string"),
                    ));
                }
            },
            None => {
                return Err(Error::new(
                    ErrorKind::InternalFailure,
                    format!("failed to get token from response"),
                ));
            }
        },
        Err(error) => {
            return Err(Error::new(
                ErrorKind::InternalFailure,
                format!("failed to deserialize token response: {}", error),
            ));
        }
    };

    Ok(token)
}
