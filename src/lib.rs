//! # RedisGo
//!
//! A simple and ergonomic Redis client wrapper for Rust.
//!
//! RedisGo provides a convenient API for common Redis operations such as
//! setting, getting, deleting keys, and more. It uses a singleton pattern
//! for easy access throughout your application.
//!
//! ## Quick Start
//!
//! Set the `REDIS_URL` environment variable or create a `.env` file:
//!
//! ```text
//! REDIS_URL=redis://127.0.0.1/
//! ```
//!
//! Then use the library:
//!
//! ```rust,no_run
//! use redisgo::RedisGo;
//!
//! fn main() -> redis::RedisResult<()> {
//!     // Set a value
//!     RedisGo::set("my_key", "my_value")?;
//!
//!     // Get a value
//!     let value: Option<String> = RedisGo::get("my_key")?;
//!     println!("Value: {:?}", value);
//!
//!     // Delete a key
//!     RedisGo::delete("my_key")?;
//!
//!     Ok(())
//! }
//! ```

use r2d2::{Pool, PooledConnection};
use redis::{cmd, Commands, Connection, FromRedisValue, RedisResult, ToRedisArgs};
use std::env;
use std::fs;
use std::sync::OnceLock;

// Lazy static singleton
static REDIS_GO: OnceLock<RedisGo> = OnceLock::new();
const DEFAULT_POOL_SIZE: u32 = 16;

/// Load `REDIS_URL` from environment or `.env` file
fn get_redis_url() -> Option<String> {
    // First check environment variable
    if let Ok(url) = env::var("REDIS_URL") {
        return Some(url);
    }

    // Fall back to .env file
    if let Ok(content) = fs::read_to_string(".env") {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == "REDIS_URL" {
                    return Some(v.trim().to_string());
                }
            }
        }
    }

    None
}

/// The main Redis client wrapper providing simplified access to Redis operations.
///
/// `RedisGo` manages a Redis connection and provides both static methods for
/// convenient access via a global singleton, and instance methods for more
/// control over the connection lifecycle.
///
/// # Example
///
/// ```rust,no_run
/// use redisgo::RedisGo;
///
/// // Using static methods (recommended for most cases)
/// RedisGo::set("key", "value").unwrap();
/// let value: Option<String> = RedisGo::get("key").unwrap();
///
/// // Using instance methods
/// use redisgo::get_redisgo;
/// let redis = get_redisgo();
/// let status = redis.get_connection_status();
/// ```
pub struct RedisGo {
    client: Option<redis::Client>,
    pool: Option<Pool<redis::Client>>,
}

impl RedisGo {
    /// Creates a new `RedisGo` instance.
    ///
    /// This method loads the `REDIS_URL` from the environment or a `.env` file
    /// and initializes the Redis client.
    ///
    /// # Errors
    ///
    /// Returns `Ok` even if the Redis URL is not set (client will be `None`).
    /// Connection errors will occur when attempting to use the client.
    pub fn new() -> RedisResult<Self> {
        let redis_url = get_redis_url();

        let client = match redis_url {
            Some(url) => redis::Client::open(url).ok(),
            None => None,
        };

        let pool = match &client {
            Some(client) => Some(Self::create_pool(client)?),
            None => None,
        };

        Ok(RedisGo { client, pool })
    }

    fn create_pool(client: &redis::Client) -> RedisResult<Pool<redis::Client>> {
        Pool::builder()
            .max_size(DEFAULT_POOL_SIZE)
            .build(client.clone())
            .map_err(|error| {
                redis::RedisError::from((
                    redis::ErrorKind::Io,
                    "Failed to build Redis connection pool",
                    error.to_string(),
                ))
            })
    }

    fn get_pool(&self) -> RedisResult<&Pool<redis::Client>> {
        self.pool.as_ref().ok_or_else(|| {
            redis::RedisError::from((redis::ErrorKind::Io, "Redis client not initialized"))
        })
    }

    fn get_connection(&self) -> RedisResult<PooledConnection<redis::Client>> {
        self.get_pool()?.get().map_err(|error| {
            redis::RedisError::from((
                redis::ErrorKind::Io,
                "Failed to get Redis connection from pool",
                error.to_string(),
            ))
        })
    }

    fn should_reconnect(error: &redis::RedisError) -> bool {
        error.is_connection_dropped() || error.is_io_error()
    }

    fn execute_operation<F, T>(
        &self,
        operation: &mut F,
    ) -> RedisResult<T>
    where
        F: FnMut(&mut Connection) -> RedisResult<T>,
    {
        let mut conn = self.get_connection()?;
        operation(&mut conn)
    }

    fn execute_with_connection<F, T>(&self, operation: F) -> RedisResult<T>
    where
        F: FnMut(&mut Connection) -> RedisResult<T>,
    {
        let mut operation = operation;

        match self.execute_operation(&mut operation) {
            Ok(result) => Ok(result),
            Err(error) if Self::should_reconnect(&error) => self.execute_operation(&mut operation),
            Err(error) => Err(error),
        }
    }

    /// Sets a key-value pair in Redis.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to set
    /// * `value` - The value to associate with the key
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis client is not initialized or the operation fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use redisgo::RedisGo;
    /// RedisGo::set("my_key", "my_value").unwrap();
    /// RedisGo::set(42_i64, 1_i64).unwrap();
    /// ```
    pub fn set<K, V>(key: K, value: V) -> RedisResult<()>
    where
        K: ToRedisArgs,
        V: ToRedisArgs,
    {
        get_redisgo().execute_with_connection(|conn| {
            cmd("SET").arg(&key).arg(&value).query::<()>(conn)
        })
    }
    /// Sets a key-value pair in Redis with a time-to-live (TTL).
    ///
    /// # Arguments
    ///
    /// * `key` - The key to set
    /// * `value` - The value to associate with the key
    /// * `ttl` - TTL in seconds.
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis client is not initialized or the operation fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use redisgo::RedisGo;
    /// // Set a key that expires in 60 seconds
    /// RedisGo::set_ex("temp_key", "temp_value", 60).unwrap();
    /// RedisGo::set_ex("session:42", vec![1_u8, 2, 3], 60).unwrap();
    /// ```
    pub fn set_ex<K, V>(key: K, value: V, ttl: u64) -> RedisResult<()>
    where
        K: ToRedisArgs,
        V: ToRedisArgs,
    {
        get_redisgo().execute_with_connection(|conn| {
            cmd("SETEX")
                .arg(&key)
                .arg(ttl)
                .arg(&value)
                .query::<()>(conn)
        })
    }

    /// Gets a value from Redis by key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(value))` if the key exists, `Ok(None)` if it doesn't.
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis client is not initialized or the operation fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use redisgo::RedisGo;
    /// let value: Option<String> = RedisGo::get("my_key").unwrap();
    /// if let Some(value) = value {
    ///     println!("Value: {}", value);
    /// }
    /// ```
    pub fn get<K, V>(key: K) -> RedisResult<V>
    where
        K: ToRedisArgs,
        V: FromRedisValue,
    {
        get_redisgo().execute_with_connection(|conn| cmd("GET").arg(&key).query(conn))
    }

    /// Deletes a key from Redis.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to delete
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis client is not initialized or the operation fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use redisgo::RedisGo;
    /// RedisGo::delete("my_key").unwrap();
    /// RedisGo::delete(42_i64).unwrap();
    /// ```
    pub fn delete<K>(key: K) -> RedisResult<()>
    where
        K: ToRedisArgs,
    {
        get_redisgo().execute_with_connection(|conn| cmd("DEL").arg(&key).query::<()>(conn))
    }

    /// Checks if a key exists in Redis.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the key exists, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis client is not initialized or the operation fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use redisgo::RedisGo;
    /// if RedisGo::exists("my_key").unwrap() {
    ///     println!("Key exists!");
    /// }
    /// ```
    pub fn exists<K>(key: K) -> RedisResult<bool>
    where
        K: ToRedisArgs,
    {
        get_redisgo().execute_with_connection(|conn| cmd("EXISTS").arg(&key).query(conn))
    }

    /// Flushes all keys from all databases.
    ///
    /// **Warning:** This will delete ALL data in Redis. Use with caution!
    /// This API is only available when the `dangerous` feature is enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis client is not initialized or the operation fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "dangerous")]
    /// # {
    /// use redisgo::RedisGo;
    /// RedisGo::flush_all().unwrap();
    /// # }
    /// ```
    #[cfg(feature = "dangerous")]
    pub fn flush_all() -> RedisResult<()> {
        get_redisgo().execute_with_connection(|conn| conn.flushall())
    }

    /// Returns a reference to the underlying Redis client.
    ///
    /// # Panics
    ///
    /// Panics if the Redis client is not initialized.
    pub fn get_client(&self) -> &redis::Client {
        self.client.as_ref().expect("Redis client not initialized")
    }

    /// Checks whether Redis currently responds to a `PING` command.
    ///
    /// This is a real liveness check rather than a pool-state check.
    pub fn is_connected(&self) -> bool {
        self.ping().is_ok()
    }

    /// Sends a PING command to Redis and returns the response.
    ///
    /// # Returns
    ///
    /// Returns "PONG" if the connection is healthy.
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis client is not initialized or the connection fails.
    pub fn ping(&self) -> RedisResult<String> {
        self.execute_with_connection(|conn| conn.ping())
    }

    /// Returns the current connection status as a human-readable string.
    pub fn get_connection_status(&self) -> String {
        if self.is_connected() {
            "Connected".to_string()
        } else {
            "Not connected".to_string()
        }
    }

    /// Returns information about the Redis client configuration.
    pub fn get_client_info(&self) -> String {
        format!("Client Info: {:?}", self.client.as_ref().map(|c| c.get_connection_info()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::cmd;
    use std::sync::{Arc, Barrier, Mutex, OnceLock};
    use std::thread;
    use std::time::{Duration, Instant};

    fn test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_concurrent_commands_use_separate_connections() {
        let _guard = test_lock().lock().unwrap();

        let redisgo = Arc::new(RedisGo::new().expect("Failed to initialize RedisGo"));
        let barrier = Arc::new(Barrier::new(2));
        let list_key = "redisgo_blocking_list";

        redisgo
            .execute_with_connection(|conn| cmd("DEL").arg(list_key).query::<usize>(conn).map(|_| ()))
            .expect("Failed to reset blocking list");

        let worker = {
            let redisgo = Arc::clone(&redisgo);
            let barrier = Arc::clone(&barrier);

            thread::spawn(move || {
                barrier.wait();
                redisgo
                    .execute_with_connection(|conn| {
                        cmd("BLPOP")
                            .arg(list_key)
                            .arg(2)
                            .query::<Option<(String, String)>>(conn)
                            .map(|_| ())
                    })
                    .expect("Failed to run blocking Redis command");
            })
        };

        barrier.wait();
        thread::sleep(Duration::from_millis(100));

        let start = Instant::now();
        let response = redisgo.ping().expect("Failed to ping Redis");
        let elapsed = start.elapsed();

        assert_eq!(response, "PONG");
        assert!(
            elapsed < Duration::from_secs(1),
            "Ping should not wait on another thread's blocking Redis command"
        );

        worker.join().unwrap();
    }
}

impl Default for RedisGo {
    fn default() -> Self {
        Self::new().expect("Failed to initialize RedisGo")
    }
}

/// Returns a reference to the global `RedisGo` singleton instance.
///
/// This function initializes the singleton on first call and returns
/// the same instance on subsequent calls.
///
/// # Panics
///
/// Panics if the `RedisGo` instance cannot be created.
///
/// # Example
///
/// ```rust,no_run
/// use redisgo::get_redisgo;
///
/// let redis = get_redisgo();
/// println!("Status: {}", redis.get_connection_status());
/// ```
pub fn get_redisgo() -> &'static RedisGo {
    REDIS_GO.get_or_init(|| RedisGo::new().expect("Failed to initialize RedisGo"))
}
