// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT

pub struct X86FunctionInstance {
    guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
    //code: &[u8],
}

#[async_trait::async_trait]
impl crate::base_runtime::FunctionInstance for X86FunctionInstance {
    async fn instantiate(
        guest_api_host: crate::base_runtime::guest_api::GuestAPIHost,
        code: &[u8],
    ) -> Result<Box<Self>, crate::base_runtime::FunctionInstanceError> {

        Ok(Box::new(Self {
            guest_api_host: guest_api_host,
            //code: code,
        }))
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
        Ok(())
    }
}
