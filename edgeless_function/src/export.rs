// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

/// Export Macro generating the functions that are exported by the WASM module and call into an instance of `EdgeFunction`.
#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "x86_64")]
#[macro_export]
macro_rules! export {
    ( $fun:ident ) => {

        #[no_mangle]
        pub unsafe extern "C" fn handle_init_asm(
            payload_ptr: *mut u8,
            payload_len: usize,
            serialized_state_ptr: *mut u8,
            serialized_state_len: usize,
        ) {
            println!("handle_init_asm paylen {} serlen {}", payload_len, serialized_state_len);
            let payload: Option<&[u8]> = if payload_len > 0 {
                Some(core::slice::from_raw_parts(payload_ptr, payload_len))
            } else {
                None
            };

            let serialized_state: Option<&[u8]> = if serialized_state_len > 0 {
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
                //let leaked_data = output_ptr
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
    }
}