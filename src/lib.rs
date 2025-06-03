use redis::{Commands, Connection, RedisResult};
use std::env;
use std::fs;
use std::sync::OnceLock;

// Lazy static singleton
static REDIS_GO: OnceLock<RedisGo> = OnceLock::new();

/// Load `.env` manually if `REDIS_URL` is not already set
fn load_env() {
    if env::var("REDIS_URL").is_err() {
        if let Ok(content) = fs::read_to_string(".env") {
            for line in content.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    env::set_var(k.trim(), v.trim());
                }
            }
        }
    }
}

pub struct RedisGo {
    client: Option<redis::Client>,
    connection: std::sync::Mutex<Option<Connection>>,
}

impl RedisGo {
    pub fn new() -> RedisResult<Self> {
        load_env();
        let redis_url = env::var("REDIS_URL").ok();

        let client = match redis_url {
            Some(url) => redis::Client::open(url).ok(),
            None => None,
        };

        Ok(RedisGo {
            client,
            connection: std::sync::Mutex::new(None),
        })
    }

    fn get_connection(&self) -> RedisResult<std::sync::MutexGuard<'_, Option<Connection>>> {
        let mut conn_guard = self.connection.lock().unwrap();
        if conn_guard.is_none() {
            if let Some(client) = &self.client {
                *conn_guard = Some(client.get_connection()?);
            } else {
                return Err(redis::RedisError::from((
                    redis::ErrorKind::IoError,
                    "Redis client not initialized",
                )));
            }
        }
        Ok(conn_guard)
    }

    fn execute_with_connection<F, T>(&self, operation: F) -> RedisResult<T>
    where
        F: FnOnce(&mut Connection) -> RedisResult<T>,
    {
        let mut conn_guard = self.get_connection()?;
        if let Some(ref mut conn) = *conn_guard {
            operation(conn)
        } else {
            Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Connection not initialized",
            )))
        }
    }

    pub fn set(key: &str, value: &str) -> RedisResult<()> {
        let redisgo = get_redisgo();
        if redisgo.client.is_none() {
            return Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Redis client not initialized",
            )));
        }
        redisgo.execute_with_connection(|conn| conn.set(key, value))
    }
    pub fn set_with_ttl(key: &str, value: &str, ttl: Option<usize>) -> RedisResult<()> {
        let redisgo = get_redisgo();
        if redisgo.client.is_none() {
            return Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Redis client not initialized",
            )));
        }
        redisgo.execute_with_connection(|conn| {
            if let Some(ttl) = ttl.map(|t| t.try_into().unwrap()) {
                conn.set_ex(key, value, ttl)
            } else {
                conn.set(key, value)
            }
        })
    }

    pub fn get(key: &str) -> RedisResult<Option<String>> {
        let redisgo = get_redisgo();
        if redisgo.client.is_none() {
            return Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Redis client not initialized",
            )));
        }
        redisgo.execute_with_connection(|conn| conn.get(key))
    }

    pub fn delete(key: &str) -> RedisResult<()> {
        let redisgo = get_redisgo();
        if redisgo.client.is_none() {
            return Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Redis client not initialized",
            )));
        }
        redisgo.execute_with_connection(|conn| conn.del(key))
    }

    pub fn exists(key: &str) -> RedisResult<bool> {
        let redisgo = get_redisgo();
        if redisgo.client.is_none() {
            return Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Redis client not initialized",
            )));
        }
        redisgo.execute_with_connection(|conn| conn.exists(key))
    }

    pub fn flush_all() -> RedisResult<()> {
        let redisgo = get_redisgo();
        if redisgo.client.is_none() {
            return Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Redis client not initialized",
            )));
        }
        redisgo.execute_with_connection(|conn| conn.flushall())
    }

    pub fn get_client(&self) -> &redis::Client {
        self.client.as_ref().expect("Redis client not initialized")
    }

    pub fn is_connected(&self) -> bool {
        self.connection.lock().unwrap().is_some()
    }

    pub fn ping(&self) -> RedisResult<String> {
        if self.client.is_none() {
            return Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Redis client not initialized",
            )));
        }
        let mut conn_guard = self.get_connection()?;
        if let Some(ref mut conn) = *conn_guard {
            conn.ping()
        } else {
            Err(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Connection not initialized",
            )))
        }
    }

    pub fn get_connection_status(&self) -> String {
        if self.is_connected() {
            "Connected".to_string()
        } else {
            "Not connected".to_string()
        }
    }

    pub fn get_client_info(&self) -> String {
        format!("Client Info: {:?}", self.client.as_ref().map(|c| c.get_connection_info()))
    }
}

pub fn get_redisgo() -> &'static RedisGo {
    REDIS_GO.get_or_init(|| RedisGo::new().expect("Failed to initialize RedisGo"))
}
