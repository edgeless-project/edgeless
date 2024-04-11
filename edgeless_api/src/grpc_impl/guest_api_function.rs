// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub struct GuestAPIFunctionClient {
    client: crate::grpc_impl::api::guest_api_function_client::GuestApiFunctionClient<tonic::transport::Channel>,
}

pub struct GuestAPIFunctionService {
    pub guest_api_function: tokio::sync::Mutex<Box<dyn crate::guest_api_function::GuestAPIFunction>>,
}

impl GuestAPIFunctionClient {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        loop {
            match crate::grpc_impl::api::guest_api_function_client::GuestApiFunctionClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Ok(Self { client });
                }
                Err(err) => match retry_interval {
                    Some(val) => tokio::time::sleep(tokio::time::Duration::from_secs(val)).await,
                    None => {
                        return Err(anyhow::anyhow!("Error when connecting to {}: {}", server_addr, err));
                    }
                },
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::guest_api_function::GuestAPIFunction for GuestAPIFunctionClient {
    async fn init(&mut self, init_data: crate::guest_api_function::FunctionInstanceInit) -> anyhow::Result<()> {
        match self.client.init(tonic::Request::new(serialize_function_instance_init(&init_data))).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while initializing function instance: {}",
                err.to_string()
            )),
        }
    }
    async fn cast(&mut self, event: crate::guest_api_function::InputEventData) -> anyhow::Result<()> {
        match self.client.cast(tonic::Request::new(serialize_input_event_data(&event))).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Communication error while casting an event: {}", err.to_string())),
        }
    }

    async fn call(&mut self, event: crate::guest_api_function::InputEventData) -> anyhow::Result<crate::guest_api_function::CallReturn> {
        match self.client.call(tonic::Request::new(serialize_input_event_data(&event))).await {
            Ok(msg) => parse_call_return(&msg.into_inner()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while calling a function instance: {}",
                err.to_string()
            )),
        }
    }
    async fn stop(&mut self) -> anyhow::Result<()> {
        match self.client.stop(tonic::Request::new(())).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while stopping function instance: {}",
                err.to_string()
            )),
        }
    }
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::guest_api_function_server::GuestApiFunction for GuestAPIFunctionService {
    async fn init(&self, init_data: tonic::Request<crate::grpc_impl::api::FunctionInstanceInit>) -> Result<tonic::Response<()>, tonic::Status> {
        let parsed_request = match parse_function_instance_init(&init_data.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing an FunctionInstanceInit message: {}",
                    err
                )));
            }
        };
        match self.guest_api_function.lock().await.init(parsed_request).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when initializing a function: {}", err))),
        }
    }

    async fn cast(&self, event: tonic::Request<crate::grpc_impl::api::InputEventData>) -> Result<tonic::Response<()>, tonic::Status> {
        let parsed_request = match parse_input_event_data(&event.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing an InputEventData message: {}",
                    err
                )));
            }
        };
        match self.guest_api_function.lock().await.cast(parsed_request).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when casting a message: {}", err))),
        }
    }

    async fn call(
        &self,
        event: tonic::Request<crate::grpc_impl::api::InputEventData>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::CallReturn>, tonic::Status> {
        let parsed_request = match parse_input_event_data(&event.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing an InputEventData message: {}",
                    err
                )));
            }
        };
        match self.guest_api_function.lock().await.call(parsed_request).await {
            Ok(msg) => Ok(tonic::Response::new(serialize_call_return(&msg))),
            Err(err) => Err(tonic::Status::internal(format!("Error when calling a function: {}", err))),
        }
    }

    async fn stop(&self, _request: tonic::Request<()>) -> Result<tonic::Response<()>, tonic::Status> {
        match self.guest_api_function.lock().await.stop().await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when stopping a function: {}", err))),
        }
    }
}

pub fn parse_function_instance_init(
    api_instance: &crate::grpc_impl::api::FunctionInstanceInit,
) -> anyhow::Result<crate::guest_api_function::FunctionInstanceInit> {
    Ok(crate::guest_api_function::FunctionInstanceInit {
        init_payload: api_instance.init_payload.clone(),
        serialized_state: api_instance.serialized_state.clone(),
    })
}

pub fn parse_input_event_data(api_instance: &crate::grpc_impl::api::InputEventData) -> anyhow::Result<crate::guest_api_function::InputEventData> {
    match &api_instance.src {
        Some(instance_id) => match crate::grpc_impl::common::CommonConverters::parse_instance_id(&instance_id) {
            Ok(src) => Ok(crate::guest_api_function::InputEventData {
                src,
                msg: api_instance.msg.clone(),
            }),
            Err(e) => Err(e),
        },
        None => Err(anyhow::anyhow!("src is missing")),
    }
}

pub fn parse_call_return(api_instance: &crate::grpc_impl::api::CallReturn) -> anyhow::Result<crate::guest_api_function::CallReturn> {
    match api_instance.r#type {
        x if x == crate::grpc_impl::api::CallRetType::CallRetNoReply as i32 => Ok(crate::guest_api_function::CallReturn::NoRet),
        x if x == crate::grpc_impl::api::CallRetType::CallRetReply as i32 => {
            Ok(crate::guest_api_function::CallReturn::Reply(api_instance.msg.clone()))
        }
        x if x == crate::grpc_impl::api::CallRetType::CallRetErr as i32 => Ok(crate::guest_api_function::CallReturn::Err),
        x => Err(anyhow::anyhow!("Ill-formed CallReturn message: unknown type {}", x)),
    }
}

fn serialize_function_instance_init(init_data: &crate::guest_api_function::FunctionInstanceInit) -> crate::grpc_impl::api::FunctionInstanceInit {
    crate::grpc_impl::api::FunctionInstanceInit {
        init_payload: init_data.init_payload.clone(),
        serialized_state: init_data.serialized_state.clone(),
    }
}

fn serialize_input_event_data(event: &crate::guest_api_function::InputEventData) -> crate::grpc_impl::api::InputEventData {
    crate::grpc_impl::api::InputEventData {
        src: Some(crate::grpc_impl::common::CommonConverters::serialize_instance_id(&event.src)),
        msg: event.msg.clone(),
    }
}

pub fn serialize_call_return(ret: &crate::guest_api_function::CallReturn) -> crate::grpc_impl::api::CallReturn {
    match ret {
        crate::guest_api_function::CallReturn::NoRet => crate::grpc_impl::api::CallReturn {
            r#type: crate::grpc_impl::api::CallRetType::CallRetNoReply as i32,
            msg: vec![],
        },
        crate::guest_api_function::CallReturn::Reply(msg) => crate::grpc_impl::api::CallReturn {
            r#type: crate::grpc_impl::api::CallRetType::CallRetReply as i32,
            msg: msg.clone(),
        },
        crate::guest_api_function::CallReturn::Err => crate::grpc_impl::api::CallReturn {
            r#type: crate::grpc_impl::api::CallRetType::CallRetErr as i32,
            msg: vec![],
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::guest_api_function::CallReturn;
    use crate::guest_api_function::FunctionInstanceInit;
    use crate::guest_api_function::InputEventData;
    use edgeless_api_core::instance_id::InstanceId;

    #[test]
    fn serialize_deserialize_function_instance_init() {
        let messages = vec![
            FunctionInstanceInit {
                init_payload: "".to_string(),
                serialized_state: vec![],
            },
            FunctionInstanceInit {
                init_payload: "init-payload".to_string(),
                serialized_state: vec![0, 42, 0, 42, 99],
            },
        ];
        for msg in messages {
            match parse_function_instance_init(&serialize_function_instance_init(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_input_event_data() {
        let messages = vec![
            InputEventData {
                src: InstanceId::new(uuid::Uuid::new_v4()),
                msg: vec![0, 42, 0, 42, 99],
            },
            InputEventData {
                src: InstanceId::none(),
                msg: vec![],
            },
        ];
        for msg in messages {
            match parse_input_event_data(&serialize_input_event_data(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_call_return() {
        let messages = vec![
            CallReturn::NoRet,
            CallReturn::Reply(vec![]),
            CallReturn::Reply(vec![0, 42, 0, 42, 99]),
            CallReturn::Err,
        ];
        for msg in messages {
            match parse_call_return(&&serialize_call_return(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }
}
