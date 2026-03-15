use redisgo::RedisGo;

// Load the .env file

#[cfg(test)]
mod tests {
    use super::*;
    use redis::cmd;
    use std::sync::{Mutex, OnceLock};

    fn test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_set_and_get() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        // Test setting a value
        RedisGo::set("test_key", "test_value").expect("Failed to set cache");

        // Test getting the value
        let value: Option<String> = RedisGo::get("test_key").expect("Failed to get cache");
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[test]
    fn test_get_nonexistent_key() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        // Test getting a nonexistent key
        let value: Option<String> = RedisGo::get("nonexistent_key").expect("Failed to get cache");
        assert_eq!(value, None);
    }

    // stress test to add and get 10000 keys
    #[test]
    fn test_stress_add_and_get() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        for i in 0..30000 {
            let key = format!("test_key_{}", i);
            let value = format!("test_value_{}", i);
            RedisGo::set(&key, &value).expect("Failed to set cache");
            let value: Option<String> = RedisGo::get(&key).expect("Failed to get cache");
            assert_eq!(value, Some(format!("test_value_{}", i)));
        }
    }

    #[test]
    fn test_typed_set_get_and_delete() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        let key = 7_i32;
        RedisGo::set(key, 42_i64).expect("Failed to set typed value");

        let value: i64 = RedisGo::get(key).expect("Failed to get typed value");
        assert_eq!(value, 42_i64);

        RedisGo::delete(key).expect("Failed to delete typed key");
        let value: Option<i64> = RedisGo::get(key).expect("Failed to get deleted typed key");
        assert_eq!(value, None);
    }

    #[test]
    fn test_delete() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        RedisGo::set("delete_key", "delete_value").expect("Failed to set cache");
        RedisGo::delete("delete_key").expect("Failed to delete cache");
        let value: Option<String> = RedisGo::get("delete_key").expect("Failed to get cache");
        assert_eq!(value, None);
    }

    #[test]
    fn test_exists() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        RedisGo::set("exists_key", "exists_value").expect("Failed to set cache");
        let exists = RedisGo::exists("exists_key").expect("Failed to check existence");
        assert!(exists);
        RedisGo::delete("exists_key").expect("Failed to delete cache");
        let exists = RedisGo::exists("exists_key").expect("Failed to check existence");
        assert!(!exists);
    }

    #[cfg(feature = "dangerous")]
    #[test]
    fn test_flush_all() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        RedisGo::set("flush_key", "flush_value").expect("Failed to set cache");
        RedisGo::flush_all().expect("Failed to flush all caches");
        let value: Option<String> = RedisGo::get("flush_key").expect("Failed to get cache");
        assert_eq!(value, None);
    }

    #[test]
    fn test_ping() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        let redisgo = RedisGo::new().expect("Failed to initialize RedisGo");
        let response = redisgo.ping().expect("Failed to ping Redis");
        assert_eq!(response, "PONG");
    }

    #[test]
    fn test_reconnects_after_cached_connection_is_killed() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        let redisgo = RedisGo::new().expect("Failed to initialize RedisGo");
        redisgo.ping().expect("Failed to establish cached connection");

        let mut admin_conn = redisgo
            .get_client()
            .get_connection()
            .expect("Failed to create admin connection");

        let killed_clients: usize = cmd("CLIENT")
            .arg("KILL")
            .arg("TYPE")
            .arg("normal")
            .arg("SKIPME")
            .arg("yes")
            .query(&mut admin_conn)
            .expect("Failed to kill cached Redis clients");

        assert!(
            killed_clients >= 1,
            "Expected cached Redis connection to be killed"
        );

        let response = redisgo
            .ping()
            .expect("Failed to transparently reconnect to Redis");
        assert_eq!(response, "PONG");
    }


    #[test]
    fn test_client_info() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        let redisgo = RedisGo::new().expect("Failed to initialize RedisGo");
        let client_info = redisgo.get_client_info();
        assert!(client_info.contains("Client Info"));
    }

    #[test]
    fn test_set_ex() {
        let _guard = test_lock().lock().unwrap_or_else(|error| error.into_inner());

        // Test setting a key with TTL
        let key = "test_key_ttl";
        let value = "test_value";
        let ttl = 10u64; // TTL of 10 seconds

        let result = RedisGo::set_ex(key, value, ttl);
        assert!(result.is_ok(), "Failed to set key with TTL");

        // Verify the key exists
        let exists = RedisGo::exists(key).unwrap();
        assert!(exists, "Key should exist after setting with TTL");

        // Wait for TTL to expire
        std::thread::sleep(std::time::Duration::from_secs(11));

        // Verify the key no longer exists
        let exists = RedisGo::exists(key).unwrap();
        assert!(!exists, "Key should not exist after TTL expiration");
    }

}
