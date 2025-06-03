# RedisGo

RedisGo is a Rust library designed to simplify interactions with Redis, providing a convenient API for common Redis operations such as setting, getting, deleting keys, and more. It leverages the `redis` crate and includes features like connection management and `.env` file support for configuration.


## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
redis = "*"
```

## Usage

### Initialization

Ensure your `.env` file contains the `REDIS_URL` variable:

```
REDIS_URL=redis://127.0.0.1/
```

### Basic Operations

#### Set a Key
```rust
RedisGo::set("key", "value")?;
```

#### Get a Key
```rust
let value = RedisGo::get("key")?;
```

#### Delete a Key
```rust
RedisGo::delete("key").unwrap();
```

#### Check if a Key Exists
```rust
let exists = RedisGo::exists("key").unwrap();
```

#### Flush All Keys
```rust
RedisGo::flush_all().unwrap();
```

### Advanced Operations

#### Set a Key with TTL
```rust
RedisGo::set_with_ttl("key", "value", Some(60)).unwrap(); // TTL in seconds
```

#### Ping Redis
```rust
let response = RedisGo::ping().unwrap();
```

#### Get Connection Status
```rust
let status = RedisGo::get_connection_status();
```

#### Get Client Info
```rust
let info = RedisGo::get_client_info();
```

### Example Usage

Here is an example of how to use the RedisGo library to implement a simple counter:

```rust
use redisgo::RedisGo;

fn main() {
    println!("Hello, world!");
    let counter_key = "counter"; 
    match RedisGo::get(counter_key) {
        Ok(Some(value)) => {
            let count: i32 = value.parse().unwrap_or(0);
            let new_count = count + 1;
            RedisGo::set(counter_key, &new_count.to_string()).unwrap();
            println!("Counter incremented to: {}", new_count);
        }
        Ok(None) => {
            RedisGo::set(counter_key, "1").unwrap();
            println!("Counter initialized to 1");
        }
        Err(e) => {
            eprintln!("Error accessing Redis: {}", e);
        }
    }
    println!("Hello, world!");
}
```

## Rusty Rails Project

Rusty Rails is a larger project aiming to bridge the gap between Rust and Ruby/Ruby on Rails. We are actively working on recreating Ruby libraries into Rust that seamlessly make working in Rust more easy and fun for new developers.

### Contributing

Contributions to the RedisGo library are welcome! Feel free to open issues, submit pull requests, or provide feedback to help improve this library.
