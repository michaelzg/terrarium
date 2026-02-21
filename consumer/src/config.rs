use log::info;
use serde::Deserialize;
use crate::db::DatabaseSettings;

#[derive(Debug, Deserialize)]
pub struct ConsumerConfig {
    pub kafka_broker: String,
    pub group_id: String,
    pub topic: String,
    pub database: DatabaseSettings,
}

impl ConsumerConfig {
    pub fn new(config_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config: Self = serde_json::from_str(config_str)?;
        info!("ConsumerConfig loaded: {:?}", config);
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::ConsumerConfig;

    #[test]
    fn parses_valid_config() {
        let json = r#"{
            "kafka_broker": "localhost:9092",
            "group_id": "consumer-group",
            "topic": "hello-topic",
            "database": {
                "host": "localhost",
                "port": 5432,
                "user": "postgres",
                "password": "postgres",
                "dbname": "messages_db",
                "pool_size": 8
            }
        }"#;

        let config = ConsumerConfig::new(json).expect("config should parse");
        assert_eq!(config.kafka_broker, "localhost:9092");
        assert_eq!(config.group_id, "consumer-group");
        assert_eq!(config.topic, "hello-topic");
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 5432);
        assert_eq!(config.database.pool_size, 8);
    }

    #[test]
    fn rejects_invalid_json() {
        let invalid = "{\"kafka_broker\": \"localhost:9092\"}";
        assert!(ConsumerConfig::new(invalid).is_err());
    }
}
