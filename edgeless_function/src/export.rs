// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

/// Export Macro generating the functions that are exported by the WASM module and call into an instance of `EdgeFunction`.
#[macro_export]
macro_rules! export {
    ( $fun:ident ) => {
        #[no_mangle]
        pub unsafe extern "C" fn handle_cast_asm(node_id_ptr: *mut u8, component_id_ptr: *mut u8, payload_ptr: *mut u8, payload_len: usize) {
            let payload: &[u8] = core::slice::from_raw_parts(payload_ptr, payload_len);
            let instance_id = InstanceId {
                node_id: core::slice::from_raw_parts(node_id_ptr, 16).try_into().unwrap(),
                component_id: core::slice::from_raw_parts(component_id_ptr, 16).try_into().unwrap(),
            };

            $fun::handle_cast(instance_id, payload);
        }

        #[no_mangle]
        pub unsafe extern "C" fn handle_call_asm(
            node_id_ptr: *mut u8,
            component_id_ptr: *mut u8,
            payload_ptr: *mut u8,
            payload_len: usize,
            out_ptr_ptr: *mut *const u8,
            out_len_ptr: *mut usize,
        ) -> i32 {
            let payload: &[u8] = core::slice::from_raw_parts(payload_ptr, payload_len);

            let instance_id = InstanceId {
                node_id: core::slice::from_raw_parts(node_id_ptr, 16).try_into().unwrap(),
                component_id: core::slice::from_raw_parts(node_id_ptr, 16).try_into().unwrap(),
            };

            let ret = $fun::handle_call(instance_id, payload);

            let (ret, output_params) = match ret {
                CallRet::NoReply => (0, None),
                CallRet::Reply(reply) => (1, Some(reply.consume())),
                CallRet::Err => (2, None),
            };
            if let (Some((output_ptr, output_len))) = output_params {
                *out_ptr_ptr = output_ptr;
                *out_len_ptr = output_len
            }
            ret
        }

        #[no_mangle]
        pub unsafe extern "C" fn handle_init_asm(
            payload_ptr: *mut u8,
            payload_len: usize,
            serialized_state_ptr: *mut u8,
            serialized_state_len: usize,
        ) {
            let payload: Option<&[u8]> = if payload_len > 0 {
                Some(core::slice::from_raw_parts(payload_ptr, payload_len))
            } else {
                None
            };

            let serialized_state = if serialized_state_len > 0 {
                Some(core::slice::from_raw_parts(serialized_state_ptr, serialized_state_len))
            } else {
                None
            };

            $fun::handle_init(payload, serialized_state);
        }

        #[no_mangle]
        pub extern "C" fn handle_stop_asm() {
            $fun::handle_stop()
        }
    };
}

/// Declare an EdgeFunction type and its constructor.
///
/// # Notes
///
/// This works by automatically generating an `extern "C"` function with a
/// pre-defined signature and symbol name. Therefore you will only be able to
/// declare one plugin per library.
#[macro_export]
macro_rules! export_x86 {
    ($plugin_type:ty, $constructor:path) => {
        #[no_mangle]
        pub unsafe extern "C" fn _plugin_create() -> *mut $crate::EdgeFunction {
            // make sure the constructor is the correct type.
            let constructor: fn() -> $plugin_type = $constructor;

            let object = constructor();
            let boxed: Box<$crate::EdgeFunction> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}

#[macro_export]
macro_rules! export_x86a {
    ( $fun:ident ) => {

        #[no_mangle]
        pub unsafe extern "C" fn handle_init_asm(
            payload_ptr: *mut u8,
            payload_len: usize,
            serialized_state_ptr: *mut u8,
            serialized_state_len: usize,
        ) {
            let payload: Option<&[u8]> = if payload_len > 0 {
                Some(core::slice::from_raw_parts(payload_ptr, payload_len))
            } else {
                None
            };

            let serialized_state = if serialized_state_len > 0 {
                Some(core::slice::from_raw_parts(serialized_state_ptr, serialized_state_len))
            } else {
                None
            };

            $fun::handle_init(payload, serialized_state)
        }

        #[no_mangle]
        pub unsafe extern "C" fn handle_call_asm(
            node_id_ptr: *mut u8,
            component_id_ptr: *mut u8,
            payload_ptr: *mut u8,
            payload_len: usize, 
            out_ptr_ptr: *mut *const u8,
            out_len_ptr: *mut usize,
        ) -> i32 {
            let payload: &[u8] = core::slice::from_raw_parts(payload_ptr, payload_len);

            let instance_id = InstanceId {
                node_id: core::slice::from_raw_parts(node_id_ptr, 16).try_into().unwrap(),
                component_id: core::slice::from_raw_parts(component_id_ptr, 16).try_into().unwrap(),
            };

            let ret = $fun::handle_call(instance_id, payload);

            let (ret, output_params) = match ret {
                CallRet::NoReply => (0, None),
                CallRet::Reply(reply) => (1, Some(reply.consume())),
                CallRet::Err => (2, None),
            };

            if let(Some((output_ptr, output_len))) = output_params {
                *out_ptr_ptr = output_ptr;
                *out_len_ptr = output_len
            }
            ret
        }

        #[no_mangle]
        pub unsafe extern "C" fn handle_cast_asm(
            node_id_ptr: *mut u8,
            component_id_ptr: *mut u8,
            payload_ptr: *mut u8,
            payload_len: usize,
        ) {
            let payload: &[u8] = core::slice::from_raw_parts(payload_ptr, payload_len);
            let instance_id = InstanceId {
                node_id: core::slice::from_raw_parts(node_id_ptr, 16).try_into().unwrap(),
                component_id: core::slice::from_raw_parts(component_id_ptr, 16).try_into().unwrap(),
            };

            $fun::handle_cast(instance_id, payload);
        }
        
        #[no_mangle]
        pub extern "C" fn handle_stop_asm() {
            $fun::handle_stop()
        }

        #[no_mangle]
        pub unsafe extern "C" fn telemetry_log_asm (
            level: usize, 
            target_ptr: *const u8, 
            target_len: usize, 
            msg_ptr: *const u8, 
            msg_len: usize,
        ) {
            //let target: String = String::from_utf8_lossy(core::slice::from_raw_parts(target_ptr, target_len)).into_owned();
            //let msg: String = String::from_utf8_lossy(core::slice::from_raw_parts(msg_ptr, msg_len)).into_owned();
            let target: &str = std::str::from_utf8(core::slice::from_raw_parts(target_ptr, target_len)).unwrap();
            let msg: &str = std::str::from_utf8(core::slice::from_raw_parts(msg_ptr, msg_len)).unwrap();

            //edgeless_node::guest_api::telemetry_log(level, target, msg);
            println!("Target: {} msg: {}", target, msg);
            
        }
    };
}