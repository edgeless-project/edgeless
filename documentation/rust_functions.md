# Rust Function API

## Function Framework

A Function needs to implement the trait `edgeless_function::api::Edgefunction`, which consists of the four callbacks shown below.
In-runtime state (not saved using sync) is currently purely the responsibility of the function (the component model does not support methods yet);
The `OnceLock` used below is one way to achieve this. The `export!` macro (last line) must be called.

```rust
use edgeless_function::api::*;

struct ExampleFunction;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ExampleState {
    count: u64,
}

static STATE: std::sync::OnceLock<std::sync::Mutex<ExampleState>> = std::sync::OnceLock::new();

impl Edgefunction for ExampleFunction {
    fn handle_cast(src: Fid, encoded_message: String) {
        log(&format!("Example: 'Cast' called, MSG: {}", encoded_message));
        STATE.get().unwrap().lock().unwrap().count += 1;
        sync(&serde_json::to_string(STATE.get().unwrap().lock().unwrap().deref()).unwrap());
    }

    fn handle_call(src: Fid, encoded_message: String) -> CallRet {
        log(&format!("Example: 'Call' called, MSG: {}", encoded_message));
        CallRet::Noreply
    }

    fn handle_init(payload: String, serialized_state: Option<String>) {
        log("Example: 'Init' called");
    }

    fn handle_stop() {
        log("Example: 'Stop' called");
    }
}

edgeless_function::export!(ExampleFunction);
```

## Available Methods

`async fn cast_alias(&mut self, alias: String, msg: String);`

Send a message to the function registered (in the workflow) for the `alias` without expecting a return value.

`async fn cast(&mut self, target: Fid, msg: String);`

Send a message to the function identified by `target` without expecting a return value.

`async fn call(&mut self, target: Fid, msg: String) -> CallRet;`

Blockingly send a message to the function identified by `target` and wait for a return value.

`async fn call_alias(&mut self, alias: String, msg: String) -> CallRet;`

Blockingly send a message to the function registered (in the workflow) for the `alias` and wait for a return value.

`async fn log(&mut self, msg: String);`

Produce a line of log.
Currently, this is just printed out; in the future, this might be sent to the monitoring system.

`async fn slf(&mut self) -> Fid;`

Retrieve the function's own `Fid` (used for continuation-passing, self-invocation etc.).

`async fn delayed_cast(&mut self, delay: u64, target: Fid, payload: String);`

After `delay` milliseconds, send a message to the function identified by `target` without expecting a return value.
This is useful for self-invocation and creating a continuously running function.

`async fn sync(&mut self, serialized_state: String);`

Write the state to disk/database (depending on the state policy).
The function is responsible for serializing the state to a string format.

## Types

```rust
struct Fid {
    node: String,
    function: String
}
```

Basic function identifier comprised of node_id & function_id.

```rust
enum CallRet {
    Reply(string),
    Noreply,
    Err
}
```

Return value from calls / value to be returned from `handle_call`.

## Project Structure

The function can be built as a `wasm32-unknown-unknown` library crate. The snippet below shows an example `Cargo.toml` file.

```toml
[workspace]

[profile.release]
lto = true
opt-level = "s"

[package]
name = "edgeless_sample_function"
version = "0.1.0"
authors = ["Raphael Hetzel <hetzel@in.tum.de"]
edition = "2021"

[lib]
name = "edgeless_sample_function"
path = "src/lib.rs"
crate-type = ["cdylib"]

[dependencies]
edgeless_function = { path = "../../../edgeless_function" }
serde = {version="1", features=["derive"] }
serde_json = "1"
```