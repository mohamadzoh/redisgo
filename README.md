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

Then, initialize the RedisGo singleton:

```rust
use redisgo::get_redisgo;

let redisgo = get_redisgo();
```

### Basic Operations

#### Set a Key
```rust
redisgo::set("key", "value")?;
```

#### Get a Key
```rust
let value = redisgo::get("key")?;
```

#### Delete a Key
```rust
redisgo::delete("key")?;
```

#### Check if a Key Exists
```rust
let exists = redisgo::exists("key")?;
```

#### Flush All Keys
```rust
redisgo::flush_all()?;
```

### Advanced Operations

#### Set a Key with TTL
```rust
redisgo::set_with_ttl("key", "value", Some(60))?; // TTL in seconds
```

#### Ping Redis
```rust
let response = redisgo.ping()?;
```

#### Get Connection Status
```rust
let status = redisgo.get_connection_status();
```

#### Get Client Info
```rust
let info = redisgo.get_client_info();
```

## Rusty Rails Project

Rusty Rails is a larger project aiming to bridge the gap between Rust and Ruby/Ruby on Rails. We are actively working on recreating Ruby libraries into Rust that seamlessly make working in Rust more easy and fun for new developers.

### Contributing

Contributions to the RedisGo library are welcome! Feel free to open issues, submit pull requests, or provide feedback to help improve this library.
