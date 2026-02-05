use redis::Client;
use std::env;

pub async fn connect() -> Result<Client, redis::RedisError> {
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    let client = Client::open(redis_url)?;
    let mut conn = client.get_multiplexed_async_connection().await?;
    let _: String = redis::cmd("PING").query_async(&mut conn).await?;
    Ok(client)
}
