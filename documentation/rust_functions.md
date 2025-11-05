# EDGELESS Rust function API

## Function Framework

A Function needs to implement the trait `edgeless_function::api::Edgefunction`, which consists of four callbacks:

- `handle_cast`: called when an asynchronous event is dispatched to this
  function instance
- `handle_call`: called when an event is dispatched to this function instance
  for which a response is expected (i.e., the caller is blocked)
- `handle_init`: called as soon as the function instance is created, before
  event handlers are called, for initialization purposes, if needed
- `handle_stop`: called before the function instance is terminated for
  clean up purposes, if needed

The naming _cast_ and _call_ have been borrowed from Erlang's [gen_server](https://www.erlang.org/doc/man/gen_server.html) terminology.

A function may have a local state, which can be wrapped inside `OnceLock`, which is a Rust synchronization primitive that ensures that the passed struct is initialized only once, but can be shared multiple times (see [explanation](https://www.dotnetperls.com/oncelock-rust)).

## Example

An example of function developed in Rust and intended for execution in the
WebAssembly run-time environment in EDGELESS is showed below.

```rust
use edgeless_function::api::*;

struct ExampleFunction;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ExampleState {
    count: u64,
}

static STATE: std::sync::OnceLock<std::sync::Mutex<ExampleState>> = std::sync::OnceLock::new();

impl Edgefunction for ExampleFunction {
    fn handle_cast(src: InstanceId, encoded_message: &[u8]) {
        log(&format!("Example: 'Cast' called, MSG: {}", encoded_message));
        STATE.get().unwrap().lock().unwrap().count += 1;
        sync(&serde_json::to_string(STATE.get().unwrap().lock().unwrap().deref()).unwrap());
    }

    fn handle_call(src: InstanceId, encoded_message: &[u8]) -> CallRet {
        log(&format!("Example: 'Call' called, MSG: {}", encoded_message));
        CallRet::Noreply
    }

    fn handle_init(payload: Option<&[u8]>, serialized_state: Option<&[u8]>) {
        log("Example: 'Init' called");
    }

    fn handle_stop() {
        log("Example: 'Stop' called");
    }
}

edgeless_function::export!(ExampleFunction);
```

## Types

Basic function identifier comprised of node identifier (`node_id`) and
physical function identifier (`function_id`):

```rust
pub struct InstanceId {
    pub node_id: NodeId,
    pub function_id: ComponentId,
}
```

Return value from calls / value to be returned from `handle_call`:

```rust
pub enum CallReturn {
    NoRet,
    Reply(Vec<u8>),
    Err,
}
```

## Available Methods

`async fn cast(&mut self, name: &str,, msg: &[u8])`

Send a message to the function registered in the workflow as `name`.
The special name `self` is reserved to send an event to the same function
instance.

`async fn call(&mut self, name: &str, msg: &[u8]) -> CallRet`

Send a message to the function registered in the workflow as `name` and wait for a response.

`async fn log(&mut self, msg: &[u8])`

Produce a line of log.

`async fn delayed_cast(&mut self, delay: u64, name: &str, msg: &[u8])`

After `delay` milliseconds, send a message to the function registered in
the workflow as `name`.

`async fn sync(&mut self, state: &[u8]);`

Write the state to disk/database, depending on the state policy.
The function is responsible for serializing the state to a string format.

## Project Structure

The function can be built as a `wasm32-unknown-unknown` (for background on
the naming [see here](https://github.com/rustwasm/wasm-bindgen/issues/979))
library crate.
The snippet below shows an example `Cargo.toml` file that can be used to build
such a function.

```ini
[workspace]

[profile.release]
lto = true
opt-level = "s"

[package]
name = "edgeless_sample_function"
version = "0.1.0"
authors = ["Raphael Hetzel <hetzel@in.tum.de>"]
edition = "2024"

[lib]
name = "edgeless_sample_function"
path = "src/lib.rs"
crate-type = ["cdylib"]

[dependencies]
edgeless_function = { path = "../../edgeless_function" }
serde = {version="1", features=["derive"] }
serde_json = "1"
```

