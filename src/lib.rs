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
//!     let value = RedisGo::get("my_key")?;
//!     println!("Value: {:?}", value);
//!
//!     // Delete a key
//!     RedisGo::delete("my_key")?;
//!
//!     Ok(())
//! }
//! ```

use redis::{Commands, Connection, RedisResult};
use std::env;
use std::fs;
use std::sync::OnceLock;

// Lazy static singleton
static REDIS_GO: OnceLock<RedisGo> = OnceLock::new();

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
/// let value = RedisGo::get("key").unwrap();
///
/// // Using instance methods
/// use redisgo::get_redisgo;
/// let redis = get_redisgo();
/// let status = redis.get_connection_status();
/// ```
pub struct RedisGo {
    client: Option<redis::Client>,
    connection: std::sync::Mutex<Option<Connection>>,
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
                    redis::ErrorKind::Io,
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
                redis::ErrorKind::Io,
                "Connection not initialized",
            )))
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
    /// ```
    pub fn set(key: &str, value: &str) -> RedisResult<()> {
        get_redisgo().execute_with_connection(|conn| conn.set(key, value))
    }
    /// Sets a key-value pair in Redis with an optional time-to-live (TTL).
    ///
    /// # Arguments
    ///
    /// * `key` - The key to set
    /// * `value` - The value to associate with the key
    /// * `ttl` - Optional TTL in seconds. If `None`, the key won't expire.
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
    /// RedisGo::set_with_ttl("temp_key", "temp_value", Some(60)).unwrap();
    /// ```
    pub fn set_with_ttl(key: &str, value: &str, ttl: Option<u64>) -> RedisResult<()> {
        get_redisgo().execute_with_connection(|conn| {
            if let Some(ttl) = ttl {
                conn.set_ex(key, value, ttl)
            } else {
                conn.set(key, value)
            }
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
    /// if let Some(value) = RedisGo::get("my_key").unwrap() {
    ///     println!("Value: {}", value);
    /// }
    /// ```
    pub fn get(key: &str) -> RedisResult<Option<String>> {
        get_redisgo().execute_with_connection(|conn| conn.get(key))
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
    /// ```
    pub fn delete(key: &str) -> RedisResult<()> {
        get_redisgo().execute_with_connection(|conn| conn.del(key))
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
    pub fn exists(key: &str) -> RedisResult<bool> {
        get_redisgo().execute_with_connection(|conn| conn.exists(key))
    }

    /// Flushes all keys from all databases.
    ///
    /// **Warning:** This will delete ALL data in Redis. Use with caution!
    ///
    /// # Errors
    ///
    /// Returns an error if the Redis client is not initialized or the operation fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use redisgo::RedisGo;
    /// RedisGo::flush_all().unwrap();
    /// ```
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

    /// Checks if a connection to Redis has been established.
    ///
    /// Note: This only checks if a connection object exists, not if the
    /// connection is still alive.
    pub fn is_connected(&self) -> bool {
        self.connection.lock().unwrap().is_some()
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
