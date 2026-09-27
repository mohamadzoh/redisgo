# RedisGo

RedisGo is a synchronous Rust library designed to simplify interactions with Redis, providing a convenient API for common Redis operations such as setting, getting, deleting keys, and more. It leverages the `redis` crate, uses an r2d2 connection pool, and is configured explicitly at startup.


## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
redisgo = "0.4.1"
```

Enable destructive helpers explicitly:

```toml
[dependencies]
redisgo = { version = "0.4.1", features = ["dangerous"] }
```

The minimum supported Rust version is 1.85. On Rust 1.85 or 1.86, pin one transitive dependency that needs a newer compiler without declaring it:

```sh
cargo update -p yoke-derive --precise 0.8.2
```

## Usage

### Initialization

Initialize the global client once with the Redis URL and pool size you want to use:

```rust
use redisgo::RedisGo;

RedisGo::init("redis://127.0.0.1/", 32).unwrap();
```

If you prefer not to use the global singleton, create an instance directly:

```rust
use redisgo::RedisGo;

let redisgo = RedisGo::new("redis://127.0.0.1/", 32).unwrap();
```

### Basic Operations

#### Set a Key
```rust
RedisGo::init("redis://127.0.0.1/", 32).unwrap();
RedisGo::set("key", "value").unwrap();
```

#### Get a Key
```rust
RedisGo::init("redis://127.0.0.1/", 32).unwrap();
let value: Option<String> = RedisGo::get("key").unwrap();
```

#### Delete a Key
```rust
RedisGo::init("redis://127.0.0.1/", 32).unwrap();
RedisGo::delete("key").unwrap();
```

#### Check if a Key Exists
```rust
RedisGo::init("redis://127.0.0.1/", 32).unwrap();
let exists = RedisGo::exists("key").unwrap();
```

#### Flush All Keys
```rust
#[cfg(feature = "dangerous")]
RedisGo::init("redis://127.0.0.1/", 32).unwrap();
RedisGo::flush_all().unwrap();
```

### Advanced Operations

#### Set a Key with TTL
```rust
RedisGo::init("redis://127.0.0.1/", 32).unwrap();
RedisGo::set_ex("key", "value", 60).unwrap(); // TTL in seconds
```

#### Typed Values
```rust
RedisGo::init("redis://127.0.0.1/", 32).unwrap();
RedisGo::set(1_i64, 42_i64).unwrap();
let value: i64 = RedisGo::get(1_i64).unwrap();
RedisGo::delete(1_i64).unwrap();
```

#### Ping Redis
```rust
let redisgo = RedisGo::new("redis://127.0.0.1/", 32).unwrap();
let response = redisgo.ping().unwrap();
```

#### Get Connection Status
```rust
let redisgo = RedisGo::new("redis://127.0.0.1/", 32).unwrap();
let status = redisgo.get_connection_status();
```

#### Get Client Info
```rust
let redisgo = RedisGo::new("redis://127.0.0.1/", 32).unwrap();
let info = redisgo.get_client_info();
```

`get_client_info()` reports connection-pool state, including max pool size and active versus idle connections.

### Example Usage

Here is an example of how to use the RedisGo library to implement a simple counter:

```rust
use redisgo::RedisGo;

fn main() {
    RedisGo::init("redis://127.0.0.1/", 32).unwrap();
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
