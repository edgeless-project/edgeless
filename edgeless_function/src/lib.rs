// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub mod lcg;

/// Provides the FunctionSpecific exports (using the crate-global `export` macro).
/// Other exported functions come from the `memory` module.
pub mod export;

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
    fn handle_cast(src: InstanceId, encoded_message: &[u8]);
    fn handle_call(src: InstanceId, encoded_message: &[u8]) -> CallRet;
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

#[cfg(feature = "std")]
pub fn init_payload_to_args(payload: Option<&[u8]>) -> std::collections::HashMap<&str, &str> {
    if let Some(payload) = payload {
        let str_payload = core::str::from_utf8(payload).unwrap();
        parse_init_payload(str_payload)
    } else {
        std::collections::HashMap::new()
    }
}

#[cfg(feature = "std")]
pub fn arg_to_bool(key: &str, arguments: &std::collections::HashMap<&str, &str>) -> bool {
    arguments.get(key).unwrap_or(&"false").to_lowercase() == "true"
}

#[cfg(feature = "std")]
pub fn arg_to_vec<T>(
    key: &str,
    pat: &str,
    arguments: &std::collections::HashMap<&str, &str>,
) -> Vec<T>
where
    T: std::str::FromStr,
{
    let value = arguments.get(key).unwrap_or(&"");
    let tokens = value.split(pat);
    tokens
        .into_iter()
        .filter_map(|x| x.parse::<T>().ok())
        .collect::<Vec<T>>()
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

    #[cfg(feature = "std")]
    #[test]
    fn test_arg_to_bool() {
        let arguments = std::collections::HashMap::from([
            ("non-bool-key", "val1"),
            ("bool-key-false", "false"),
            ("bool-key-true-1", "true"),
            ("bool-key-true-2", "True"),
            ("bool-key-true-3", "TRUE"),
        ]);

        assert!(!arg_to_bool("non-bool-key", &arguments));
        assert!(!arg_to_bool("bool-key-false", &arguments));
        assert!(arg_to_bool("bool-key-true-1", &arguments));
        assert!(arg_to_bool("bool-key-true-2", &arguments));
        assert!(arg_to_bool("bool-key-true-3", &arguments));
        assert!(!arg_to_bool("non-existing-key", &arguments));
        assert!(!arg_to_bool("", &arguments));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_arg_to_vec() {
        let arguments = std::collections::HashMap::from([
            ("vec1", "1:2:3:4:5"),
            ("vec2", "1@2@3@4@5"),
            ("vec3", "3.4:6.8"),
            ("vec4", "1:-2:3:-4:5"),
            ("vec5", "1:two:3:four:5"),
        ]);

        assert_eq!(
            vec![1, 2, 3, 4, 5],
            arg_to_vec::<usize>("vec1", ":", &arguments)
        );
        assert!(arg_to_vec::<usize>("vec1", "@", &arguments).is_empty());
        assert_eq!(
            vec![1, 2, 3, 4, 5],
            arg_to_vec::<u32>("vec1", ":", &arguments)
        );
        assert_eq!(
            vec![1, 2, 3, 4, 5],
            arg_to_vec::<i32>("vec1", ":", &arguments)
        );
        assert_eq!(
            vec![1, 2, 3, 4, 5],
            arg_to_vec::<usize>("vec2", "@", &arguments)
        );
        assert_eq!(vec![3.4, 6.8], arg_to_vec::<f32>("vec3", ":", &arguments));
        assert_eq!(vec![1, 3, 5], arg_to_vec::<usize>("vec4", ":", &arguments));
        assert_eq!(
            vec![1, -2, 3, -4, 5],
            arg_to_vec::<i32>("vec4", ":", &arguments)
        );
        assert_eq!(vec![1, 3, 5], arg_to_vec::<usize>("vec5", ":", &arguments));
        assert!(arg_to_vec::<usize>("non-existing", "@", &arguments).is_empty());
    }
}
