// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

use chrono::ParseWeekdayError;

use libloading::{Library, Symbol};
use serde_json::value::to_raw_value;
use std::fs::File;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::prelude::*;
use futures::TryFutureExt;

use crate::base_runtime::guest_api::GuestAPIError;
use edgeless_function::memory;

type HandleInitFun = fn (*mut u8, usize,  *mut u8, usize);
type HandleCallFun = fn (*mut u8, *mut u8, *mut u8, usize, *mut *const u8, *mut usize) -> i32;
type HandleCastFun = fn (*mut u8, *mut u8, *mut u8, usize);
type HandleStopFun = fn ();
type SetGuestAPIHostPointer = extern "C" fn (*const usize);

pub struct NativeFunctionInstance {
    guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
    library: Library, 
    instance_id: edgeless_api::function_instance::InstanceId,
    code_file_path: PathBuf,
    native_runtime_api: Option<Box<dyn edgeless_api::native_runtime::NativeRuntimeAPI + Send>>,
}

fn level_from_i32(lvl: i32) -> edgeless_telemetry::telemetry_events::TelemetryLogLevel {
    match lvl {
        1 => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Error,
        2 => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Warn,
        3 => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info,
        4 => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Debug,
        5 => edgeless_telemetry::telemetry_events::TelemetryLogLevel::Trace,
        _ => {
            log::warn!("Function used unknown Log Level");
            edgeless_telemetry::telemetry_events::TelemetryLogLevel::Error
        }
    }
}

#[async_trait::async_trait]
impl crate::base_runtime::FunctionInstance for NativeFunctionInstance {
    async fn instantiate(
        instance_id: &edgeless_api::function_instance::InstanceId,
        _runtime_configuration: std::collections::HashMap<String, String>,
        guest_api_host: &mut Option<crate::base_runtime::guest_api::GuestAPIHost>,
        code: &[u8],
    ) -> Result<Box<Self>, crate::base_runtime::FunctionInstanceError> {

        let file_path = format!("/tmp/native-{}.so", instance_id.function_id);
        log::debug!("Native RT (instantiate): file path: {} code len: {}", file_path, code.len());
        let code_path = Path::new(file_path.as_str());

        let mut file = match File::create(code_path) { 
            Ok(file) => file,
            Err(e) => { 
                log::info!("Native RT: Cannot open file /tmp/native.so error {}", e); 
                panic!("couldn't create {}: {}", code_path.display(), e)
                },
        };
        match file.write_all(code) {
            Ok(_) => {},
            Err(e) => {
                log::info!("Native RT: Cannot write code to file error: {}.", e);
                panic!("coudn't write to {}: {}", code_path.display(), e)
            },
        }

        unsafe{
            let lib = Library::new(code_path).unwrap();
            
            Ok(Box::new(Self {
                guest_api_host: guest_api_host.take().expect("No GuestAPIHost"),
                library: lib, 
                instance_id: instance_id.to_owned(),
                code_file_path: code_path.to_owned(),
                native_runtime_api: None,  
            }))
        }
    }

    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&str>) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        let init_payload_str = init_payload.unwrap_or_default();
        let init_payload_str_len = init_payload_str.len();
        let init_payload_str_ptr = init_payload_str.as_ptr();

        let serialized_state_str = serialized_state.unwrap_or_default();
        let serialized_state_str_len = serialized_state_str.len();
        let serialized_state_str_ptr = serialized_state_str.as_ptr();

        unsafe {
            let set_guest_api_host_pointer: Symbol<SetGuestAPIHostPointer> = self.library.get(b"set_guest_api_host_pointer").unwrap();
            let ptr = self as *const NativeFunctionInstance as usize;
            log::debug!("Native RT. Setting self pointer {:p}", ptr as *const usize); 
            set_guest_api_host_pointer(ptr as *const usize);

            let handle_init_fun: Symbol<HandleInitFun> = self.library.get(b"handle_init_asm").unwrap();
            handle_init_fun(init_payload_str_ptr as *mut u8, init_payload_str_len, 
                serialized_state_str_ptr as *mut u8, serialized_state_str_len);
        }
                
        Ok(())
    }

    async fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        msg: &str
    ) -> Result<edgeless_dataplane::core::CallRet, crate::base_runtime::FunctionInstanceError> {

        let payload_len = msg.as_bytes().len();

        unsafe {
            let handle_call_fun: Symbol<HandleCallFun> = self.library.get(b"handle_call_asm").unwrap();

            let node_id = src.node_id.as_bytes();
            let component_id = src.function_id.as_bytes();

            let mut out_ptr_ptr: *const u8 = std::ptr::null();
            let mut out_len_ptr: usize = 0;

            let callret_type = handle_call_fun(
                node_id.as_ptr() as *mut u8,
                component_id.as_ptr() as *mut u8,
                msg.as_ptr() as *mut u8,
                payload_len,
                &mut out_ptr_ptr,
                &mut out_len_ptr,
            );

            let ret = match callret_type {
                0 => Ok(edgeless_dataplane::core::CallRet::NoReply),
                1 => {
                    let reply_raw = std::slice::from_raw_parts(out_ptr_ptr, out_len_ptr);
                    let reply = std::string::String::from_utf8_unchecked(reply_raw.to_vec()); 
                    // deallocate the memory that was allocated inside the called function
                    // not sure we can actually do it as data are just converted to slice, not copied
                    edgeless_function::memory::edgeless_mem_free(out_ptr_ptr as *mut u8, out_len_ptr);
                    Ok(edgeless_dataplane::core::CallRet::Reply(reply))
                }
                _ => Ok(edgeless_dataplane::core::CallRet::Err),
            };

            ret
        }
    }


    async fn cast(&mut self, src: &edgeless_api::function_instance::InstanceId, msg: &str) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        let payload_len = msg.as_bytes().len();
        unsafe {
            let handle_cast_fun: Symbol<HandleCastFun> = self.library.get(b"handle_cast_asm").unwrap();

            let node_id = src.node_id.as_bytes();
            let component_id = src.function_id.as_bytes();

            let node_id_str = std::str::from_utf8_unchecked(node_id);
            let component_id_str = std::str::from_utf8_unchecked(component_id);
            handle_cast_fun(
                node_id.as_ptr() as *mut u8, 
                component_id.as_ptr() as *mut u8,
                msg.as_ptr() as *mut u8,
                payload_len
            );
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::info!("Native RT (stop): About to stop the function.");
        unsafe {
            let handle_stop_fun: Symbol<HandleStopFun> = self.library.get(b"handle_stop_asm").unwrap();
            handle_stop_fun();
            log::info!("Function stopped.");
        }

        log::info!("Native RT: deleting file {}", self.code_file_path.display());

        match fs::remove_file(self.code_file_path.as_path()) {
            Ok(_) => {},
            Err(e) => {
                log::info!("Native RT: Cannot delete file {} {} with error {}", 
                    self.code_file_path.to_string_lossy(), self.code_file_path.display(), 
                    e);
            }
        }
        Ok(())
    }

}

impl NativeFunctionInstance {
    #[no_mangle]
    unsafe extern "C" fn cast_raw_asm(
        &mut self, 
        instance_node_id_ptr: *const u8, 
        instance_component_id_ptr: *const u8, 
        payload_ptr: *const u8, 
        payload_len: usize
    ) {
        let mut node_id: [u8; 16] = [0; 16];
        let mut component_id: [u8; 16] = [0; 16];
        unsafe {
            std::ptr::copy_nonoverlapping(instance_node_id_ptr, node_id.as_mut_ptr(), 16);
            std::ptr::copy_nonoverlapping(instance_component_id_ptr, component_id.as_mut_ptr(), 16);
        }
        
        let instance_id = edgeless_api::function_instance::InstanceId{
            node_id: uuid::Uuid::from_bytes(node_id),
            function_id: uuid::Uuid::from_bytes(component_id)
        };

        let payload: &str = std::str::from_utf8(core::slice::from_raw_parts(payload_ptr, payload_len)).unwrap();

        let future = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.guest_api_host.call_raw(instance_id, payload).await
            })
        });

        //let handle = tokio::runtime::Handle::current();
        //let _ = handle.enter();
        //futures::executor::block_on(self.guest_api_host.call_raw(instance_id, payload));
    }

    #[no_mangle]
    unsafe extern "C" fn cast_asm(
        &mut self,
        target_ptr: *const u8, 
        target_len: usize, 
        payload_ptr: *const u8, 
        payload_len: usize
    ) {
        let target: &str = std::str::from_utf8(core::slice::from_raw_parts(target_ptr, target_len)).unwrap();
        let payload: &str = std::str::from_utf8(core::slice::from_raw_parts(payload_ptr, payload_len)).unwrap();

        //println!("Native RT: Cast: Target: {} Payload: {}", target, payload);
        
        let future = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.guest_api_host.cast_alias(target, payload).await
            })
        });

        //let handle = tokio::runtime::Handle::current();
        //let _ = handle.enter();
        //futures::executor::block_on(self.guest_api_host.cast_alias(target, payload));
    }

    #[no_mangle]
    unsafe extern "C" fn call_raw_asm(
        &mut self,
        instance_node_id_ptr: *const u8,
        instance_component_id_ptr: *const u8,
        payload_ptr: *const u8,
        payload_len: usize,
        out_ptr_ptr: *mut *mut u8,
        out_len_ptr: *mut usize,
    ) -> i32 {
        let mut node_id: [u8; 16] = [0; 16];
        let mut component_id: [u8; 16] = [0; 16];
        unsafe {
            std::ptr::copy_nonoverlapping(instance_node_id_ptr, node_id.as_mut_ptr(), 16);
            std::ptr::copy_nonoverlapping(instance_component_id_ptr, component_id.as_mut_ptr(), 16);
        }
        
        let instance_id = edgeless_api::function_instance::InstanceId{
            node_id: uuid::Uuid::from_bytes(node_id),
            function_id: uuid::Uuid::from_bytes(component_id)
        };

        let payload: &str = std::str::from_utf8(core::slice::from_raw_parts(payload_ptr, payload_len)).unwrap();

        let callret_type = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.guest_api_host
                    .call_raw(instance_id, payload,
                    ).await  
            })
        });

        match callret_type {
            Ok(ret) => match ret {
                edgeless_dataplane::core::CallRet::NoReply => { log::debug!("Native RT Reply no data"); 0 },
                edgeless_dataplane::core::CallRet::Reply(data) => { log::debug!("Native RT Reply data: {}", data); 1 }
                edgeless_dataplane::core::CallRet::Err => { log::debug!("Native RT Reply Err"); 2 },

            },
            Err(GuestAPIError) => { log::info!("Native RT Call Raw GuestAPIError"); 3 },
        };

        0
    }

    #[no_mangle]    
    unsafe extern "C" fn call_asm(
        &mut self,
        target_ptr: *const u8,
        target_len: usize,
        payload_ptr: *const u8,
        payload_len: usize,
        out_ptr_ptr: *mut *mut u8,
        out_len_ptr: *mut usize,
    ) -> i32 {
        let target: &str = std::str::from_utf8(core::slice::from_raw_parts(target_ptr, target_len)).unwrap();
        let payload: &str = std::str::from_utf8(core::slice::from_raw_parts(payload_ptr, payload_len)).unwrap();

        println!("Native RT: Call: Target: {} Payload: {}", target, payload);
        
       
        let callret_type = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on( async {
            self.guest_api_host
                .call_alias(
                    target,
                    payload,
                ).await
            })
        });

        match callret_type {
            Ok(ret) => match ret { 
                edgeless_dataplane::core::CallRet::NoReply => { log::debug!("Native RT Reply with no data"); 0 },
                edgeless_dataplane::core::CallRet::Reply(data) => { 
                    log::debug!("Native RT Reply with data: {}", data);

                    // we need to leak the data so they are accessible in the edgeless_function
                    // memory should be freed in edgeless_function after use
                    // TODO! fix the memory leak by freeing the memory
                    let leaked_data = data.clone().leak();
                    
                    let data_ptr = leaked_data.as_mut_ptr();
                    *out_ptr_ptr = data_ptr;
                    *out_len_ptr = data.len();
                    1
                },
                edgeless_dataplane::core::CallRet::Err => { log::debug!("Native RT Reply with err"); 2 },
            },
            Err(GuestAPIError) => { log::info!("Native RT GuesAPIError"); 3 },
        }

         
    }

    #[no_mangle] 
    unsafe extern "C" fn telemetry_log_asm (
        &mut self,
        level: usize, 
        target_ptr: *const u8, 
        target_len: usize, 
        msg_ptr: *const u8, 
        msg_len: usize,
    ) {
        let lvl = level_from_i32(level as i32);
        let target: &str = std::str::from_utf8(core::slice::from_raw_parts(target_ptr, target_len)).unwrap();
        let msg: &str = std::str::from_utf8(core::slice::from_raw_parts(msg_ptr, msg_len)).unwrap();

        log::info!("Native RT: Log: Level: {} Target: {} msg: {}", level, target, msg);

        let future = tokio::task::block_in_place(|| { 
            tokio::runtime::Handle::current().block_on( async {
                self.guest_api_host.telemetry_log(lvl, target, msg).await
            })
        });
    }
    
    #[no_mangle]
    unsafe extern "C" fn slf_asm(
        &mut self, 
        out_node_id_ptr: *mut u8, 
        out_component_id_ptr: *mut u8
    ) {
        let handle = tokio::runtime::Handle::current();
        let _ = handle.enter();
        let instance_id = futures::executor::block_on(self.guest_api_host.slf());
        
        let node_id_bytes = instance_id.node_id.as_bytes();
        std::ptr::copy_nonoverlapping(node_id_bytes.as_ptr(), out_node_id_ptr, node_id_bytes.len());

        let function_id_bytes = instance_id.function_id.as_bytes();
        std::ptr::copy_nonoverlapping(function_id_bytes.as_ptr(), out_component_id_ptr, function_id_bytes.len());
        
        log::debug!("Native RT: slf id: {}", instance_id);
    }

    #[no_mangle]
    unsafe extern "C" fn delayed_cast_asm(
        &mut self,
        delay_ms: u64, 
        target_ptr: *const u8, 
        target_len: usize, 
        payload_ptr: *const u8, 
        payload_len: usize
    ) {
        let target: &str = std::str::from_utf8(core::slice::from_raw_parts(target_ptr, target_len)).unwrap();
        let payload: &str = std::str::from_utf8(core::slice::from_raw_parts(payload_ptr, payload_len)).unwrap();
        
        log::debug!("Native RT: Delayed Cast: Delay: {} Target: {} Payload: {}", delay_ms, target, payload);
        
        let future = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on( async {
                self.guest_api_host.delayed_cast(delay_ms, &target, &payload).await
            })
        });

        //let handle = tokio::runtime::Handle::current();
        //let _ = handle.enter();
        //futures::executor::block_on(self.guest_api_host.delayed_cast(delay_ms, &target, &payload));
    }

    #[no_mangle]
    unsafe extern "C" fn sync_asm(
        &mut self, 
        data_ptr: *const u8, 
        data_len: usize
    ) {
        let state: &str = std::str::from_utf8(core::slice::from_raw_parts(data_ptr, data_len)).unwrap();

        log::debug!("Native RT: state: {}", state);
        
        let future = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.guest_api_host.sync(state).await
            })
        });

        //let handle = tokio::runtime::Handle::current();
        //let _ = handle.enter();
        //futures::executor::block_on(self.guest_api_host.sync(state));
    }
}
