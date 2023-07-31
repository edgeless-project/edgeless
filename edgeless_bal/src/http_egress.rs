pub struct EgressResourceProvider {
    resource_provider_id: edgeless_api::function_instance::FunctionId,
    dataplane_provider: edgeless_dataplane::DataPlaneChainProvider,
    egress_instances: std::collections::HashMap<edgeless_api::function_instance::FunctionId, EgressResource>,
}

pub struct EgressResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for EgressResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl EgressResource {
    async fn new(dataplane_handle: edgeless_dataplane::DataPlaneChainHandle) -> Self {
        let mut dataplane_handle = dataplane_handle;

        let handle = tokio::spawn(async move {
            loop {
                let (fid, channel, message) = dataplane_handle.receive_next().await;
                if channel == 0 {
                    continue;
                }
                let mut dataplane_write_handle = dataplane_handle.new_write_handle().await;
                let req = match edgeless_http::request_from_string(&message) {
                    Ok(val) => val,
                    Err(_) => {
                        dataplane_write_handle.reply(fid, channel, edgeless_dataplane::CallRet::Err).await;
                        continue;
                    }
                };
                tokio::spawn(async move {
                    match Self::perform_request(req).await {
                        Ok(resp) => {
                            let serialized_resp = edgeless_http::response_to_string(&resp);
                            dataplane_write_handle
                                .reply(fid, channel, edgeless_dataplane::CallRet::Reply(serialized_resp))
                                .await;
                        }
                        Err(_) => {
                            dataplane_write_handle.reply(fid, channel, edgeless_dataplane::CallRet::Err).await;
                        }
                    }
                });
            }
        });

        Self { join_handle: handle }
    }

    async fn perform_request(req: edgeless_http::EdgelessHTTPRequest) -> anyhow::Result<edgeless_http::EdgelessHTTPResponse> {
        let method = reqwest::Method::from_bytes(edgeless_http::edgeless_method_to_string(req.method).as_bytes())?;

        let protocol_string = match req.protocol {
            edgeless_http::EdgelessHTTPProtocol::HTTPS => "HTTPS",
            _ => "HTTP",
        };

        let url = format!("{}://{}{}", protocol_string, req.host, req.path);

        let client = reqwest::Client::new();

        let mut client_r = client.request(method, url);

        if let Some(b) = req.body {
            client_r = client_r.body(b);
        }

        for (header_key, header_val) in req.headers {
            client_r = client_r.header(header_key, header_val);
        }

        let ret = client_r.send().await?;

        let headers = ret
            .headers()
            .iter()
            .filter_map(|(k, v)| match v.to_str() {
                Ok(value) => Some((k.to_string(), value.to_string())),
                _ => {
                    log::warn!("Could not parse received header value");
                    None
                }
            })
            .collect();

        Ok(edgeless_http::EdgelessHTTPResponse {
            status: ret.status().as_u16(),
            headers: headers,
            body: match ret.bytes().await {
                Ok(btes) => Some(btes.to_vec()),
                _ => None,
            },
        })
    }
}

impl EgressResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::DataPlaneChainProvider,
        resource_provider_id: edgeless_api::function_instance::FunctionId,
    ) -> Self {
        Self {
            resource_provider_id,
            dataplane_provider,
            egress_instances: std::collections::HashMap::<edgeless_api::function_instance::FunctionId, EgressResource>::new(),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI for EgressResourceProvider {
    async fn start_resource_instance(
        &mut self,
        _instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::function_instance::FunctionId> {
        let new_id = edgeless_api::function_instance::FunctionId::new(self.resource_provider_id.node_id);
        let dataplane_handle = self.dataplane_provider.get_chain_for(new_id.clone()).await;

        self.egress_instances.insert(new_id.clone(), EgressResource::new(dataplane_handle).await);

        Ok(new_id)
    }

    async fn stop_resource_instance(&mut self, resource_id: edgeless_api::function_instance::FunctionId) -> anyhow::Result<()> {
        self.egress_instances.remove(&resource_id);
        Ok(())
    }
}
