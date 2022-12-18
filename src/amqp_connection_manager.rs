use std::sync::Arc;

use crate::config::amqp_connection_manager_config::AmqpConnectionManagerConfig;
use lapin::{Channel, Connection, ConnectionProperties};
use tokio::sync::Mutex;

use crate::error::{Error, ErrorKind};

pub struct AmqpConnectionManager {
    config: AmqpConnectionManagerConfig,
    connections: Arc<Mutex<Vec<Connection>>>,
}

impl AmqpConnectionManager {
    pub async fn try_new(
        config: AmqpConnectionManagerConfig,
    ) -> Result<AmqpConnectionManager, Error> {
        let connection = AmqpConnectionManager::amqp_connect(&config).await?;

        Ok(AmqpConnectionManager {
            config,
            connections: Arc::new(Mutex::new(vec![connection])),
        })
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
        let mut connections = match self.connections.try_lock() {
            Ok(connections) => connections,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::InternalFailure,
                    format!("failed to lock connections: {}", error),
                ))
            }
        };

        for connection in connections.as_slice() {
            match connection.create_channel().await {
                Ok(channel) => return Ok(channel),
                Err(error) => {
                    if error != lapin::Error::ChannelsLimitReached {
                        return Err(Error::new(
                            ErrorKind::ApiConnectionFailure,
                            format!("failed to create channel: {}", error),
                        ));
                    }
                }
            }
        }

        let connection = AmqpConnectionManager::amqp_connect(&self.config).await?;
        let channel = match connection.create_channel().await {
            Ok(channel) => channel,
            Err(error) => {
                return Err(Error::new(
                    ErrorKind::ApiConnectionFailure,
                    format!("failed to create channel: {}", error),
                ));
            }
        };

        connections.push(connection);
        Ok(channel)
    }
}
