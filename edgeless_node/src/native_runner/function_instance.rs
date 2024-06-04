// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

//extern crate libloading;

//use std::collections::HashMap;
use libloading::{Library, Symbol};

type HandleStopFun = fn ();

pub struct NativeFunctionInstance {
    guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
    //code: &[u8],
    library: Library, 
    //edgefunctione_handle_stop: Symbol<'a, HandleStopFun>,
    //edgefunctione_handle_stop: Symbol<HandleStopFun>,
}

#[async_trait::async_trait]
impl crate::base_runtime::FunctionInstance for NativeFunctionInstance {
    async fn instantiate(
        _instance_id: &edgeless_api::function_instance::InstanceId,
        _runtime_configuration: std::collections::HashMap<String, String>,
        guest_api_host: &mut Option<crate::base_runtime::guest_api::GuestAPIHost>,
        code: &[u8],
    ) -> Result<Box<Self>, crate::base_runtime::FunctionInstanceError> {
        unsafe{
            let lib = Library::new("/home/roman/workspace/edgeless/examples/noop/noop_function/noop.so").unwrap();

            //let handle_stop_fun: Symbol<HandleStopFun> = lib.get::<HandleStopFun>(b"handle_stop_asm").unwrap().clone();
            let handle_stop_fun: Symbol<HandleStopFun> = lib.get(b"handle_stop_asm").unwrap();
        
            Ok(Box::new(Self {
                guest_api_host: guest_api_host.take().expect("No GuestAPIHost"),
                //code: code,
                library: lib, 
                //edgefunctione_handle_stop: handle_stop_fun,
            }))
        }
    }

    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&str>) -> Result<(), crate::base_runtime::FunctionInstanceError> {
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

    async fn cast(&mut self, src: &edgeless_api::function_instance::InstanceId, msg: &str) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        let payload_len = msg.as_bytes().len();

        Ok(())
    }

    async fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        msg: &str
    ) -> Result<edgeless_dataplane::core::CallRet, crate::base_runtime::FunctionInstanceError> {
        let payload_len = msg.as_bytes().len();

        Ok(edgeless_dataplane::core::CallRet::NoReply)
    }

    async fn stop(&mut self) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::info!("About to stop the function.");
        unsafe {
            //let handle_stop_fun: Symbol<HandleStopFun> = lib.get::<HandleStopFun>(b"handle_stop_asm").unwrap().clone();
            let handle_stop_fun: Symbol<HandleStopFun> = self.library.get(b"handle_stop_asm").unwrap();
            handle_stop_fun();
            log::info!("Function stopped.");
        }
        Ok(())
    }
}
