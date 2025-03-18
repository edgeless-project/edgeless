// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

//extern crate libloading;

//use std::collections::HashMap;
use libloading::{Library, Symbol};
use std::fs::File;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::prelude::*;

//type HandleInitFun = fn (&str, &str);
type HandleInitFun = fn (&str, usize,  &str, usize);
//type HandleInitFun = fn (&[u8], usize,  &[u8], usize);
type HandleCallFun = fn (*mut u8, *mut u8, *mut u8, usize, *mut *const u8, *mut usize) -> i32;
type HandleCastFun = fn (*mut u8, *mut u8, *mut u8, usize);
type HandleStopFun = fn ();
type SetGuestAPIHostPointer = extern "C" fn (*const usize);

pub struct NativeFunctionInstance {
    guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
    //function_client_api: Box<dyn edgeless_api::guest_api_function::GuestAPIFunction>,
    //code: &[u8],
    library: Library, 
    //edgefunctione_handle_stop: Symbol<'a, HandleStopFun>,
    //edgefunctione_handle_stop: Symbol<HandleStopFun>,
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

/*#[no_mangle]
pub unsafe extern "C" fn telemetry_log_asm (
    guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
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
    println!("Native RT: Log: Target: {} msg: {}", target, msg);

    
    //guest_api_host.telemetry_log(edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info, target, msg);    
}*/

#[async_trait::async_trait]
impl crate::base_runtime::FunctionInstance for NativeFunctionInstance {
    async fn instantiate(
        instance_id: &edgeless_api::function_instance::InstanceId,
        _runtime_configuration: std::collections::HashMap<String, String>,
        guest_api_host: &mut Option<crate::base_runtime::guest_api::GuestAPIHost>,
        code: &[u8],
    ) -> Result<Box<Self>, crate::base_runtime::FunctionInstanceError> {
        //log::info!("Native RT (instantiate): code: {}", String::from_utf8(code.to_vec()).unwrap());
        log::info!("Native RT (instantiate): code: {}", code.len());

        let file_path = format!("/tmp/native-{}.so", instance_id.function_id);
        println!("{file_path}");
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

        //let native_runtime_api = crate::native_runner::native_runtime::NativeRuntimeClient::new(Box::new(guest_api_host)); //.clone())); //take().expect("No GuestAPIHost")));

        unsafe{
            let lib = Library::new(code_path).unwrap();

            //let handle_stop_fun: Symbol<HandleStopFun> = lib.get::<HandleStopFun>(b"handle_stop_asm").unwrap().clone();
            //let handle_stop_fun: Symbol<HandleStopFun> = lib.get(b"handle_stop_asm").unwrap();
        
            Ok(Box::new(Self {
                guest_api_host: guest_api_host.take().expect("No GuestAPIHost"),
                //function_client_api: None, 
                //code: code,
                library: lib, 
                //edgefunctione_handle_stop: handle_stop_fun,
                instance_id: instance_id.to_owned(),
                code_file_path: code_path.to_owned(),
                native_runtime_api: None,  
            }))
        }
    }

    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&str>) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::info!(
            "Native RT (init): payload: {}, serialized_state {} bytes", 
            init_payload.unwrap_or_default(), 
            serialized_state.unwrap_or_default().len()
        );
        unsafe {
            let set_guest_api_host_pointer: Symbol<SetGuestAPIHostPointer> = self.library.get(b"set_guest_api_host_pointer").unwrap();
            let ptr = self as *const NativeFunctionInstance as usize;
            println!("Nativer RT. Setting self pointer {:p}", ptr as *const usize); 
            set_guest_api_host_pointer(ptr as *const usize);

            let handle_init_fun: Symbol<HandleInitFun> = self.library.get(b"handle_init_asm").unwrap();
            //handle_init_fun(init_payload.unwrap_or(&""), serialized_state.unwrap_or_default());
            handle_init_fun(init_payload.unwrap_or(&""), init_payload.unwrap_or(&"").len(), 
            serialized_state.unwrap_or_default(), serialized_state.unwrap_or_default().len());
        }
        /*let (init_payload_ptr, init_payload_len) = match init_payload {
            Some(payload) => {
                let len = payload.len();
                let ptr = &payload;
                (ptr, len as i32)
            }

            None => (0i32, 0i32),
        };*/
        
        Ok(())
    }

    async fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        msg: &str
    ) -> Result<edgeless_dataplane::core::CallRet, crate::base_runtime::FunctionInstanceError> {

        return Ok(edgeless_dataplane::core::CallRet::NoReply);

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
                //std::str::from_utf8(src.node_id.as_bytes()).unwrap(),
                //std::str::from_utf8(src.function_id.as_bytes()).unwrap(),
                msg.as_ptr() as *mut u8,
                payload_len
            );
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::info!("Native RT (stop): About to stop the function.");
        unsafe {
            //let handle_stop_fun: Symbol<HandleStopFun> = lib.get::<HandleStopFun>(b"handle_stop_asm").unwrap().clone();
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

    /*fn set_native_runtime_api(&mut self, native_runtime_api: Box<dyn edgeless_api::native_runtime::NativeRuntimeAPI + Send>)  {
        self.native_runtime_api = native_runtime_api;
    }*/

    /*async fn sync_asm(store: wasmtime::Caller<'_, GuestAPI>, state_ptr: &str, state_len: usize) {
        Box::new(super::guest_api_binding::sync(store, state_ptr, state_len))
            .map_err(|_| crate::base_runtime::FunctionInstanceError::InternalError)?;
    }*/
}

impl NativeFunctionInstance {
    #[no_mangle]
    unsafe extern "C" fn cast_raw_asm(
        &mut self, 
        instance_node_id_ptr: *const u8, 
        instance_component_id_ptr: *const u8, 
        payload_ptr: *const u8, payload_len: usize
    ) {

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

        println!("Native RT: Cast: Target: {} Payload: {}", target, payload);
        
        let handle = tokio::runtime::Handle::current();
        let _ = handle.enter();
        futures::executor::block_on(self.guest_api_host.cast_alias(target, payload));
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
        
        let handle = tokio::runtime::Handle::current();
        let _ = handle.enter();
        let call_res = futures::executor::block_on(self.guest_api_host.call_alias(target, payload));
        
        0
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

        println!("Native RT: Log: Level: {} Target: {} msg: {}", level, target, msg);

        let handle = tokio::runtime::Handle::current();
        let _ = handle.enter();

        futures::executor::block_on(self.guest_api_host.telemetry_log(lvl, target, msg));

        //self.guest_api_host.telemetry_log(lvl, target, msg).await;
        
        //tokio::runtime::Handle::current().block_on(self.guest_api_host.telemetry_log(lvl, target, msg));
        //self.guest_api_host.telemetry_log(lvl, target, msg).wait();
        //self.native_runtime_client.telemetry_log(edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info, target, msg);
        //let telemetry_log_event = TelemetryLogEvent(edgeless_api_core::instance_id::InstanceId edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info, target, msg);
        //self.native_runtime_client.telemetry_log(telemetry_log_event);
            
    }
    
    #[no_mangle]
    unsafe extern "C" fn slf_asm(
        &mut self, 
        out_node_id_ptr: *mut u8, 
        out_component_id_ptr: *mut u8
    ) {
        //let id = tokio::runtime::Handle::current().block_on(self.guest_api_host.slf());
        let handle = tokio::runtime::Handle::current();
        let _ = handle.enter();
        let instance_id = futures::executor::block_on(self.guest_api_host.slf());
        
        let node_id_bytes = instance_id.node_id.as_bytes();
        std::ptr::copy_nonoverlapping(node_id_bytes.as_ptr(), out_node_id_ptr, node_id_bytes.len());

        let function_id_bytes = instance_id.function_id.as_bytes();
        std::ptr::copy_nonoverlapping(function_id_bytes.as_ptr(), out_component_id_ptr, function_id_bytes.len());
        
        println!("Native RT: slf id: {}", instance_id);
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
        
        println!("Native RT: Delayed Cast: Delay: {} Target: {} Payload: {}", delay_ms, target, payload);
        
        let handle = tokio::runtime::Handle::current();
        let _ = handle.enter();
        futures::executor::block_on(self.guest_api_host.delayed_cast(delay_ms, &target, &payload));
    }

    #[no_mangle]
    unsafe extern "C" fn sync_asm(
        &mut self, 
        data_ptr: *const u8, 
        data_len: u32
    ) {

    }
}
