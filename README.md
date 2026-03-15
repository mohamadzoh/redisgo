# RedisGo

RedisGo is a Rust library designed to simplify interactions with Redis, providing a convenient API for common Redis operations such as setting, getting, deleting keys, and more. It leverages the `redis` crate and includes features like connection management and `.env` file support for configuration.


## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
redisgo = "0.3.0"
```

Enable destructive helpers explicitly:

```toml
[dependencies]
redisgo = { version = "0.3.0", features = ["dangerous"] }
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
let value: Option<String> = RedisGo::get("key")?;
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
#[cfg(feature = "dangerous")]
RedisGo::flush_all().unwrap();
```

### Advanced Operations

#### Set a Key with TTL
```rust
RedisGo::set_ex("key", "value", 60).unwrap(); // TTL in seconds
```

#### Typed Values
```rust
RedisGo::set(1_i64, 42_i64)?;
let value: i64 = RedisGo::get(1_i64)?;
RedisGo::delete(1_i64)?;
```

#### Ping Redis
```rust
let redisgo = RedisGo::new().unwrap();
let response = redisgo.ping().unwrap();
```

#### Get Connection Status
```rust
let redisgo = RedisGo::new().unwrap();
let status = redisgo.get_connection_status();
```

#### Get Client Info
```rust
let redisgo = RedisGo::new().unwrap();
let info = redisgo.get_client_info();
```

### Example Usage

Here is an example of how to use the RedisGo library to implement a simple counter:

```rust
use redisgo::RedisGo;

fn main() {
    println!("Hello, world!");
    let counter_key = "counter"; 
    match RedisGo::get::<_, Option<i32>>(counter_key) {
        Ok(Some(value)) => {
            let count = value;
            let new_count = count + 1;
            RedisGo::set(counter_key, new_count).unwrap();
            println!("Counter incremented to: {}", new_count);
        }
        Ok(None) => {
            RedisGo::set(counter_key, 1_i32).unwrap();
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
