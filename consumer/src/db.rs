pub use deadpool_postgres::{Pool, Config, ManagerConfig, RecyclingMethod, Runtime};
use serde::Deserialize;
use tokio_postgres::NoTls;

#[derive(Debug, Deserialize)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
    pub pool_size: usize,
}

pub async fn create_pool(settings: &DatabaseSettings) -> Result<Pool, Box<dyn std::error::Error>> {
    log::info!("Creating database pool with host={}, port={}, dbname={}, user={}", 
        settings.host, settings.port, settings.dbname, settings.user);

    let mut cfg = Config::new();
    cfg.host = Some(settings.host.clone());
    cfg.port = Some(settings.port);
    cfg.user = Some(settings.user.clone());
    cfg.password = Some(settings.password.clone());
    cfg.dbname = Some(settings.dbname.clone());
    
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    
    cfg.pool = Some(deadpool_postgres::PoolConfig {
        max_size: settings.pool_size,
        ..Default::default()
    });

    let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
    
    // Test the connection
    let client = pool.get().await?;
    let row = client.query_one("SELECT version()", &[]).await?;
    let version: String = row.get(0);
    log::info!("Successfully connected to PostgreSQL: {}", version);
    
    Ok(pool)
}

pub async fn insert_message(
    pool: &Pool,
    topic: &str,
    partition: i32,
    offset: i64,
    payload: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    log::debug!("Attempting to insert message - Topic: {}, Partition: {}, Offset: {}", 
        topic, partition, offset);

    let mut client = pool.get().await?;
    
    // Start a transaction
    let tx = client.transaction().await?;
    
    match tx.execute(
        "INSERT INTO messages (topic, part, kafkaoffset, payload) VALUES ($1, $2, $3, $4)",
        &[&topic, &partition, &offset, &payload],
    ).await {
        Ok(rows) => {
            tx.commit().await?;
            log::debug!("Successfully inserted {} row(s) into messages table", rows);
            Ok(())
        },
        Err(e) => {
            tx.rollback().await?;
            log::error!("Failed to insert message into database: {}", e);
            Err(e.into())
        }
    }
}
