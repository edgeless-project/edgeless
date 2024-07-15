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
type HandleCallFun = fn (&str, &str, &str, usize, *mut *const u8, *mut usize);
type HandleCastFun = fn (&str, &str, &str, usize);
type HandleStopFun = fn ();

pub struct NativeFunctionInstance {
    guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
    //function_client_api: Box<dyn edgeless_api::guest_api_function::GuestAPIFunction>,
    //code: &[u8],
    library: Library, 
    //edgefunctione_handle_stop: Symbol<'a, HandleStopFun>,
    //edgefunctione_handle_stop: Symbol<HandleStopFun>,
    instance_id: edgeless_api::function_instance::InstanceId,
    code_file_path: PathBuf,
}

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
        let payload_len = msg.as_bytes().len();

        unsafe {
            let handle_call_fun: Symbol<HandleCallFun> = self.library.get(b"handle_call_asm").unwrap();
            let mut out_ptr_ptr: *const u8 = std::ptr::null();
            let mut_out_ptr_ptr: *mut *const u8 = &mut out_ptr_ptr; 
            let mut out_len_ptr: usize = 0;

            handle_call_fun(
                std::str::from_utf8(self.instance_id.node_id.as_bytes()).unwrap(),
                std::str::from_utf8(self.instance_id.node_id.as_bytes()).unwrap(),
                msg,
                payload_len,
                mut_out_ptr_ptr,
                &mut out_len_ptr,
            );
        }

        Ok(edgeless_dataplane::core::CallRet::NoReply)
    }


    async fn cast(&mut self, src: &edgeless_api::function_instance::InstanceId, msg: &str) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        let payload_len = msg.as_bytes().len();
        unsafe {
            let handle_cast_fun: Symbol<HandleCastFun> = self.library.get(b"handle_cast_asm").unwrap();

            handle_cast_fun(
                std::str::from_utf8(self.instance_id.node_id.as_bytes()).unwrap(),
                std::str::from_utf8(self.instance_id.node_id.as_bytes()).unwrap(),
                msg,
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
        match fs::remove_file(self.code_file_path.as_path()) {
            Ok(_) => {},
            Err(e) => {
                log::info!("Native RT: Cannot delete file {} with error {}", 
                    self.code_file_path.to_string_lossy(), 
                    e);
            }
        }
        Ok(())
    }
}
