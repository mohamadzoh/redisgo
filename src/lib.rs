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
//! Initialize the client once with the Redis URL and pool size, then use the
//! library anywhere in your application:
//!
//! ```rust,no_run
//! use redisgo::RedisGo;
//!
//! fn main() -> redis::RedisResult<()> {
//!     RedisGo::init("redis://127.0.0.1/", 32)?;
//!
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
use std::sync::OnceLock;

// Lazy static singleton
static REDIS_GO: OnceLock<RedisGo> = OnceLock::new();
pub const DEFAULT_POOL_SIZE: u32 = 16;

fn invalid_config(message: &'static str, detail: impl Into<String>) -> redis::RedisError {
    redis::RedisError::from((redis::ErrorKind::InvalidClientConfig, message, detail.into()))
}

fn validate_redis_url(redis_url: impl Into<String>) -> RedisResult<String> {
    let redis_url = redis_url.into();
    let redis_url = redis_url.trim();

    if redis_url.is_empty() {
        return Err(invalid_config(
            "Missing Redis configuration",
            "redis_url must not be empty",
        ));
    }

    Ok(redis_url.to_string())
}

fn validate_pool_size(pool_size: u32) -> RedisResult<u32> {
    if pool_size == 0 {
        return Err(invalid_config(
            "Invalid Redis pool size",
            "pool_size must be greater than zero",
        ));
    }

    Ok(pool_size)
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
/// RedisGo::init("redis://127.0.0.1/", 16).unwrap();
///
/// // Using static methods (recommended for most cases)
/// RedisGo::set("key", "value").unwrap();
/// let value: Option<String> = RedisGo::get("key").unwrap();
///
/// // Using instance methods
/// let redis = RedisGo::new("redis://127.0.0.1/", 16).unwrap();
/// let status = redis.get_connection_status();
/// ```
pub struct RedisGo {
    redis_url: String,
    pool: Pool<redis::Client>,
}

impl RedisGo {
    /// Creates a new `RedisGo` instance.
    ///
    /// This method initializes the Redis client from the provided arguments.
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis URL is invalid, the pool size is zero, or the
    /// connection pool cannot be created.
    pub fn new(redis_url: impl Into<String>, pool_size: u32) -> RedisResult<Self> {
        let redis_url = validate_redis_url(redis_url)?;
        let pool_size = validate_pool_size(pool_size)?;
        let client = redis::Client::open(redis_url.clone())?;
        let pool = Self::create_pool(&client, pool_size)?;

        Ok(RedisGo { redis_url, pool })
    }

    /// Initializes the global `RedisGo` singleton.
    ///
    /// Repeated calls with the same configuration return the existing instance.
    /// Calling this with a different configuration after initialization returns an error.
    pub fn init(redis_url: impl Into<String>, pool_size: u32) -> RedisResult<&'static Self> {
        let redis_url = validate_redis_url(redis_url)?;
        let pool_size = validate_pool_size(pool_size)?;

        if let Some(existing) = REDIS_GO.get() {
            if existing.redis_url == redis_url && existing.pool.max_size() == pool_size {
                return Ok(existing);
            }

            return Err(invalid_config(
                "RedisGo already initialized",
                format!(
                    "existing config uses redis_url={} and pool_size={}",
                    existing.redis_url,
                    existing.pool.max_size()
                ),
            ));
        }

        let redisgo = Self::new(redis_url, pool_size)?;
        let _ = REDIS_GO.set(redisgo);

        Ok(REDIS_GO
            .get()
            .expect("RedisGo should be initialized after a successful init call"))
    }

    fn create_pool(client: &redis::Client, pool_size: u32) -> RedisResult<Pool<redis::Client>> {
        Pool::builder()
            .max_size(pool_size)
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
        Ok(&self.pool)
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
        self.execute_operation(&mut operation)
    }

    fn execute_with_retry<F, T>(&self, operation: F) -> RedisResult<T>
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
            cmd("SET")
                .arg(&key)
                .arg(&value)
                .arg("EX")
                .arg(ttl)
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
    /// Returns the value decoded as `V`.
    /// Use `Option<T>` when the key may be missing.
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
        get_redisgo().execute_with_retry(|conn| cmd("GET").arg(&key).query(conn))
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
        get_redisgo().execute_with_retry(|conn| cmd("EXISTS").arg(&key).query(conn))
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

    /// Returns a newly constructed Redis client using the current configuration.
    ///
    /// # Panics
    ///
    /// Panics if the stored Redis URL is invalid.
    pub fn get_client(&self) -> redis::Client {
        redis::Client::open(self.redis_url.clone()).expect("Redis client not initialized")
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
        self.execute_with_retry(|conn| conn.ping())
    }

    /// Returns the current connection status as a human-readable string.
    pub fn get_connection_status(&self) -> String {
        if self.is_connected() {
            "Connected".to_string()
        } else {
            "Not connected".to_string()
        }
    }

    /// Returns information about the Redis connection pool.
    pub fn get_client_info(&self) -> String {
        let state = self.pool.state();
        format!(
            "Pool Info: max_size={}, connections={}, idle_connections={}",
            self.pool.max_size(),
            state.connections,
            state.idle_connections
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::cmd;
    use std::net::{SocketAddr, TcpStream};
    use std::sync::{Arc, Barrier, Mutex, OnceLock};
    use std::thread;
    use std::time::{Duration, Instant};

    fn test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn redis_available() -> bool {
        let address: SocketAddr = "127.0.0.1:6379".parse().expect("Invalid test Redis address");
        TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok()
    }

    #[test]
    fn test_concurrent_commands_use_separate_connections() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        if !redis_available() {
            return;
        }

        let redisgo = Arc::new(
            RedisGo::new("redis://127.0.0.1/", DEFAULT_POOL_SIZE)
                .expect("Failed to initialize RedisGo"),
        );
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

    #[test]
    fn test_new_rejects_empty_redis_url() {
        let error = RedisGo::new("   ", DEFAULT_POOL_SIZE)
            .err()
            .expect("Expected an invalid configuration error");
        assert_eq!(error.kind(), redis::ErrorKind::InvalidClientConfig);
    }

    #[test]
    fn test_new_rejects_zero_pool_size() {
        let error = RedisGo::new("redis://127.0.0.1/", 0)
            .err()
            .expect("Expected an invalid configuration error");
        assert_eq!(error.kind(), redis::ErrorKind::InvalidClientConfig);
    }

    #[test]
    fn test_init_returns_existing_instance_for_same_config() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        if !redis_available() {
            return;
        }

        let first = RedisGo::init("redis://127.0.0.1/", DEFAULT_POOL_SIZE)
            .expect("Failed to initialize RedisGo") as *const RedisGo;
        let second = RedisGo::init("redis://127.0.0.1/", DEFAULT_POOL_SIZE)
            .expect("Failed to reuse initialized RedisGo") as *const RedisGo;

        assert_eq!(first, second);
    }
}

/// Returns a reference to the global `RedisGo` singleton instance.
///
/// Call `RedisGo::init(redis_url, pool_size)` before using this function.
///
/// # Example
///
/// ```rust,no_run
/// use redisgo::{get_redisgo, RedisGo};
///
/// RedisGo::init("redis://127.0.0.1/", 16).unwrap();
/// let redis = get_redisgo();
/// println!("Status: {}", redis.get_connection_status());
/// ```
pub fn get_redisgo() -> &'static RedisGo {
    REDIS_GO.get().expect(
        "RedisGo is not initialized. Call RedisGo::init(redis_url, pool_size) before using global operations",
    )
}
