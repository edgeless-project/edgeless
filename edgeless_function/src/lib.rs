// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub mod lcg;

/// Guest Codegen based on function.json
pub use edgeless_function_macro::generate;

/// These functions are imported by the WASM module.
pub mod imports;

/// Provides a memory managment wrapper for data that was passed by the host and must be freed outside of the internal functions of this crate.
/// Mostly exists to only require one abstraction for both std and no_std mode.
pub mod owned_data;
pub use owned_data::OwnedByteBuff;

/// Provides a log-crate-compatible logger passing the logs the host.
/// The reexported `init_logger` function must be called (e.g., in the init function) for the `log` macros the work.
pub mod logging;
pub use logging::init_logger;

/// Provides the memory management functions required by the host to pass data to the WASM environment.
/// These functions are exported by the WASM module.
pub mod memory;

/// Provides the (reeported) functions that enable the edgeless actors to interact with the outside world.
pub mod output_api;
pub use output_api::*;

pub enum CallRet {
    NoReply,
    Reply(owned_data::OwnedByteBuff),
    Err,
}

pub struct InstanceId {
    /// UUID node_id
    pub node_id: [u8; 16],
    /// UUID component_id
    pub component_id: [u8; 16],
}

pub trait EdgeFunction {
    fn handle_cast(src: InstanceId, port: &str, encoded_message: &[u8]);
    fn handle_call(src: InstanceId, port: &str, encoded_message: &[u8]) -> CallRet;
    fn handle_init(payload: Option<&[u8]>, _serialized_state: Option<&[u8]>);
    fn handle_stop();
}

#[cfg(feature = "std")]
pub fn parse_init_payload(payload: &str) -> std::collections::HashMap<&str, &str> {
    let tokens = payload.split(',');
    let mut arguments = std::collections::HashMap::new();
    for token in tokens {
        let mut inner_tokens = token.split('=');
        if let Some(key) = inner_tokens.next() {
            if let Some(value) = inner_tokens.next() {
                arguments.insert(key, value);
            } else {
                log::error!("invalid initialization token: {}", token);
            }
        } else {
            log::error!("invalid initialization token: {}", token);
        }
    }
    arguments
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(feature = "std")]
    #[test]
    fn test_parse_init_payload() {
        assert_eq!(
            std::collections::HashMap::from([("a", "b"), ("c", "d"), ("my_key", "my_value")]),
            parse_init_payload("a=b,c=d,my_key=my_value")
        );

        assert_eq!(
            std::collections::HashMap::from([("a", ""), ("", "d"), ("my_key", "my_value")]),
            parse_init_payload("a=,=d,my_key=my_value")
        );

        assert_eq!(
            std::collections::HashMap::from([("my_key", "my_value")]),
            parse_init_payload("a,d,my_key=my_value")
        );

        assert!(parse_init_payload(",,,a,s,s,,42,").is_empty());
    }
}
