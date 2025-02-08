use log::info;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ConsumerConfig {
    pub kafka_broker: String,
    pub group_id: String,
    pub topic: String,
}

impl ConsumerConfig {
    pub fn new(config_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config: Self = serde_json::from_str(config_str)?;
        info!("ConsumerConfig loaded: {:?}", config);
        Ok(config)
    }
}
