
use crate::error::{Error, ErrorKind};

pub struct AmqpConnectionManagerConfig {
    connection_uri: String,
}

impl AmqpConnectionManagerConfig {
    pub fn new(connection_uri: String) -> AmqpConnectionManagerConfig {
        AmqpConnectionManagerConfig { connection_uri }
    }

    pub fn connection_uri(&self) -> String {
        self.connection_uri.clone()
    }
}

const AMQP_API_CONNECTION_URI: &str = "AMQP_API_CONNECTION_URI";

pub fn try_generate_config() -> Result<AmqpConnectionManagerConfig, Error> {
    let connection_uri = match std::env::var(AMQP_API_CONNECTION_URI) {
        Ok(connection_uri) => connection_uri,
        Err(error) => {
            return Err(Error::new(
                ErrorKind::InternalFailure,
                format!("failed to read environment variable: {}", error),
            ));
        }
    };

    Ok(AmqpConnectionManagerConfig::new(connection_uri))
}
