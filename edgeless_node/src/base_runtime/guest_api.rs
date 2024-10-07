// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use futures::FutureExt;

/// Each function instance can import a set of functions that need to be implemented on the host-side.
/// This provides the generic host-side implementation of these functions.
/// Those need to be made available to the guest using a virtualization-specific interface/binding.
pub struct GuestAPIHost {
    pub instance_id: edgeless_api::function_instance::InstanceId,
    pub data_plane: edgeless_dataplane::handle::DataplaneHandle,
    pub state_handle: Box<dyn crate::state_management::StateHandleAPI>,
    pub telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    pub poison_pill_receiver: tokio::sync::broadcast::Receiver<()>,
}

/// Errors to be reported by the host side of the guest binding.
/// This may need to be bridged into the runtime by the virtualization-specific runtime implementation.
#[derive(Debug)]
pub enum GuestAPIError {
    UnknownAlias,
}

impl GuestAPIHost {
    pub async fn cast_alias(&mut self, alias: &str, msg: &str) -> Result<(), GuestAPIError> {
        log::info!("dp send");
        self.data_plane
            .send_alias(alias.to_string(), msg.to_string())
            .await
            .map_err(|e| GuestAPIError::UnknownAlias)
    }

    pub async fn cast_raw(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        target_port: edgeless_api::function_instance::PortId,
        msg: &str,
    ) -> Result<(), GuestAPIError> {
        self.data_plane.send(target, target_port, msg.to_string()).await;
        Ok(())
    }

    pub async fn call_alias(&mut self, alias: &str, msg: &str) -> Result<edgeless_dataplane::core::CallRet, GuestAPIError> {
        Ok(self.data_plane.call_alias(alias.to_string(), msg.to_string()).await)
    }

    pub async fn call_raw(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        target_port: edgeless_api::function_instance::PortId,
        msg: &str,
    ) -> Result<edgeless_dataplane::core::CallRet, GuestAPIError> {
        futures::select! {
            _ = Box::pin(self.poison_pill_receiver.recv()).fuse() => {
                return Ok(edgeless_dataplane::core::CallRet::Err)
            },
            call_res = Box::pin(self.data_plane.call(target, target_port, msg.to_string())).fuse() => {
                return Ok(call_res)
            }
        }
    }

    pub async fn telemetry_log(&mut self, lvl: edgeless_telemetry::telemetry_events::TelemetryLogLevel, target: &str, msg: &str) {
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionLogEntry(lvl, target.to_string(), msg.to_string()),
            std::collections::BTreeMap::new(),
        );
    }

    pub async fn slf(&mut self) -> edgeless_api::function_instance::InstanceId {
        self.instance_id.clone()
    }

    pub async fn delayed_cast(&mut self, delay: u64, target_alias: &str, payload: &str) -> Result<(), GuestAPIError> {
        let mut cloned_plane = self.data_plane.clone();
        let cloned_msg = payload.to_string();
        let cloned_alias = target_alias.to_string();

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            cloned_plane.send_alias(cloned_alias, cloned_msg).await.unwrap();
        });

        Ok(())
    }

    pub async fn sync(&mut self, serialized_state: &str) -> Result<(), GuestAPIError> {
        self.state_handle.set(serialized_state.to_string()).await;
        log::info!("Function State Sync: {}", serialized_state);
        Ok(())
    }
}
