// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_api::container_function::ContainerFunctionAPI;

/// FunctionInstance implementation allowing to execute functions defined
/// as computational containers through a gRPC API.
pub struct ContainerFunctionInstance {
    /// gRPC function client to interact with the container function.
    _function_client: edgeless_api::grpc_impl::container_function::ContainerFunctionAPIClient,
    /// Protocol-neutral API to interact with the container function.
    function_client_api: Box<dyn edgeless_api::guest_api_function::GuestAPIFunction>,
}

#[async_trait::async_trait]
impl crate::base_runtime::FunctionInstance for ContainerFunctionInstance {
    async fn instantiate(
        runtime_configuration: std::collections::HashMap<String, String>,
        _guest_api_host: &mut Option<crate::base_runtime::guest_api::GuestAPIHost>,
        code: &[u8],
    ) -> Result<Box<Self>, crate::base_runtime::FunctionInstanceError> {
        let fun_spec = String::from_utf8(code.to_vec()).unwrap_or_default();
        log::info!("container run-time: instantiate {}", fun_spec);

        // Assume the fun_spec is one of (examples):
        // - container:hello-world
        // - grpc:http://127.0.0.1:1234
        if let Some((fun_type, fun_addr)) = fun_spec.split_once(':') {
            if fun_type != "grpc" {
                log::error!("container function type not implemented: {}", fun_type);
                return Err(crate::base_runtime::FunctionInstanceError::BadCode);
            }
            match edgeless_api::grpc_impl::container_function::ContainerFunctionAPIClient::new(fun_addr, None).await {
                Ok(mut _function_client) => {
                    let mut function_client_api = _function_client.guest_api_function();

                    match runtime_configuration.get("guest_api_host_url") {
                        Some(url) => {
                            match function_client_api
                                .boot(edgeless_api::guest_api_function::BootData {
                                    guest_api_host_endpoint: url.clone(),
                                })
                                .await
                            {
                                Ok(_) => Ok(Box::new(Self {
                                    _function_client,
                                    function_client_api,
                                })),
                                Err(err) => {
                                    log::error!("could not boot the container function instance: {}", err);
                                    Err(crate::base_runtime::FunctionInstanceError::InternalError)
                                }
                            }
                        }
                        None => {
                            log::error!("invalid or missing guest_api_host_url");
                            Err(crate::base_runtime::FunctionInstanceError::InternalError)
                        }
                    }
                }
                Err(err) => {
                    log::error!("could not connect to the function instance at {}: {}", fun_addr, err);
                    Err(crate::base_runtime::FunctionInstanceError::InternalError)
                }
            }
        } else {
            log::error!("invalid container function specifier: {}", fun_spec);
            Err(crate::base_runtime::FunctionInstanceError::BadCode)
        }
    }

    async fn init(&mut self, init_payload: Option<&str>, serialized_state: Option<&str>) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::debug!(
            "container run-time: init, payload {}, serialized_state {} bytes",
            init_payload.unwrap_or_default(),
            serialized_state.unwrap_or_default().len()
        );
        self.function_client_api
            .init(edgeless_api::guest_api_function::FunctionInstanceInit {
                init_payload: init_payload.unwrap_or(&"").to_string(),
                serialized_state: serialized_state.unwrap_or(&"").as_bytes().to_vec(),
            })
            .await
            .or(Err(crate::base_runtime::FunctionInstanceError::InternalError))
    }

    async fn cast(&mut self, src: &edgeless_api::function_instance::InstanceId, msg: &str) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::debug!("container run-time: cast, src {}, msg {} bytes", src, msg.len());
        log::info!("XXX container run-time: cast, src {}, msg {}", src, msg);
        self.function_client_api
            .cast(edgeless_api::guest_api_function::InputEventData {
                src: src.clone(),
                msg: msg.into(),
            })
            .await
            .or(Err(crate::base_runtime::FunctionInstanceError::InternalError))
    }

    async fn call(
        &mut self,
        src: &edgeless_api::function_instance::InstanceId,
        msg: &str,
    ) -> Result<edgeless_dataplane::core::CallRet, crate::base_runtime::FunctionInstanceError> {
        log::debug!("container run-time: call, src {}, msg {} bytes", src, msg.len());
        match self
            .function_client_api
            .call(edgeless_api::guest_api_function::InputEventData {
                src: src.clone(),
                msg: msg.into(),
            })
            .await
        {
            Ok(ret) => match ret {
                edgeless_api::guest_api_function::CallReturn::NoRet => Ok(edgeless_dataplane::core::CallRet::NoReply),
                edgeless_api::guest_api_function::CallReturn::Reply(msg) => {
                    Ok(edgeless_dataplane::core::CallRet::Reply(String::from_utf8(msg).unwrap_or_default()))
                }
                edgeless_api::guest_api_function::CallReturn::Err => Ok(edgeless_dataplane::core::CallRet::Err),
            },
            Err(_) => Err(crate::base_runtime::FunctionInstanceError::InternalError),
        }
    }

    async fn stop(&mut self) -> Result<(), crate::base_runtime::FunctionInstanceError> {
        log::debug!("container run-time: stop");
        self.function_client_api
            .stop()
            .await
            .or(Err(crate::base_runtime::FunctionInstanceError::InternalError))
    }
}
