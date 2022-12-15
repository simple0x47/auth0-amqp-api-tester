use lapin::{Channel, Connection, ConnectionProperties};
use crate::config::amqp_connection_manager_config::AmqpConnectionManagerConfig;

use crate::error::{Error, ErrorKind};

pub struct AmqpConnectionManager {
    config: AmqpConnectionManagerConfig,
    connection: Connection,
}

impl AmqpConnectionManager {
    pub async fn try_new(
        config: AmqpConnectionManagerConfig,
    ) -> Result<AmqpConnectionManager, Error> {
        let connection = AmqpConnectionManager::amqp_connect(&config).await?;

        Ok(AmqpConnectionManager { config, connection })
    }

    async fn amqp_connect(config: &AmqpConnectionManagerConfig) -> Result<Connection, Error> {
        let connection_options = ConnectionProperties::default()
            .with_executor(tokio_executor_trait::Tokio::current())
            .with_reactor(tokio_reactor_trait::Tokio);

        let connection =
            match Connection::connect(config.connection_uri().as_str(), connection_options).await {
                Ok(connection) => connection,
                Err(error) => {
                    return Err(Error::new(
                        ErrorKind::ApiConnectionFailure,
                        format!("API connection failure: {}", error),
                    ));
                }
            };

        Ok(connection)
    }

    pub async fn try_get_channel(&self) -> Result<Channel, Error> {
        let channel = match self.connection.create_channel().await {
            Ok(channel) => channel,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::ApiConnectionFailure,
                    format!("failed to create channel: {}", error),
                ));
            }
        };

        Ok(channel)
    }
}
