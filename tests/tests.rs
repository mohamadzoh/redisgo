use redisgo::RedisGo;

// Load the .env file

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {

        // Test setting a value
        RedisGo::set("test_key", "test_value").expect("Failed to set cache");

        // Test getting the value
        let value = RedisGo::get("test_key").expect("Failed to get cache");
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[test]
    fn test_get_nonexistent_key() {

        // Test getting a nonexistent key
        let value = RedisGo::get("nonexistent_key").expect("Failed to get cache");
        assert_eq!(value, None);
    }

    // stress test to add and get 10000 keys
    #[test]
    fn test_stress_add_and_get() {
        for i in 0..30000 {
            let key = format!("test_key_{}", i);
            let value = format!("test_value_{}", i);
            RedisGo::set(&key, &value).expect("Failed to set cache");
            let value = RedisGo::get(&key).expect("Failed to get cache");
            assert_eq!(value, Some(format!("test_value_{}", i)));
        }
    }

    #[test]
    fn test_delete() {
        RedisGo::set("delete_key", "delete_value").expect("Failed to set cache");
        RedisGo::delete("delete_key").expect("Failed to delete cache");
        let value = RedisGo::get("delete_key").expect("Failed to get cache");
        assert_eq!(value, None);
    }

    #[test]
    fn test_exists() {
        RedisGo::set("exists_key", "exists_value").expect("Failed to set cache");
        let exists = RedisGo::exists("exists_key").expect("Failed to check existence");
        assert!(exists);
        RedisGo::delete("exists_key").expect("Failed to delete cache");
        let exists = RedisGo::exists("exists_key").expect("Failed to check existence");
        assert!(!exists);
    }

    #[test]
    fn test_flush_all() {
        RedisGo::set("flush_key", "flush_value").expect("Failed to set cache");
        RedisGo::flush_all().expect("Failed to flush all caches");
        let value = RedisGo::get("flush_key").expect("Failed to get cache");
        assert_eq!(value, None);
    }

    #[test]
    fn test_ping() {
        let redisgo = RedisGo::new().expect("Failed to initialize RedisGo");
        let response = redisgo.ping().expect("Failed to ping Redis");
        assert_eq!(response, "PONG");
    }


    #[test]
    fn test_client_info() {
        let redisgo = RedisGo::new().expect("Failed to initialize RedisGo");
        let client_info = redisgo.get_client_info();
        assert!(client_info.contains("Client Info"));
    }

    #[test]
    fn test_set_with_ttl() {

        // Test setting a key with TTL
        let key = "test_key_ttl";
        let value = "test_value";
        let ttl = Some(10); // TTL of 10 seconds

        let result = RedisGo::set_with_ttl(key, value, ttl);
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

    #[test]
    fn test_set_without_ttl() {
        // Test setting a key without TTL
        let key = "test_key_no_ttl";
        let value = "test_value";

        let result = RedisGo::set_with_ttl(key, value, None);
        assert!(result.is_ok(), "Failed to set key without TTL");

        // Verify the key exists
        let exists = RedisGo::exists(key).unwrap();
        assert!(exists, "Key should exist after setting without TTL");

        // Verify the key still exists after some time
        std::thread::sleep(std::time::Duration::from_secs(5));
        let exists = RedisGo::exists(key).unwrap();
        assert!(exists, "Key should still exist without TTL");
    }
}
