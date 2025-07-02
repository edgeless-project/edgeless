// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use futures::FutureExt;

/// Each function instance can import a set of functions that need to be implemented on the host-side.
/// This provides the generic host-side implementation of these functions.
/// Those need to be made available to the guest using a virtualization-specific interface/binding.
pub struct GuestAPIHost {
    pub instance_id: edgeless_api::function_instance::InstanceId,
    pub data_plane: edgeless_dataplane::handle::DataplaneHandle,
    pub callback_table: crate::base_runtime::alias_mapping::AliasMapping,
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
        if alias == "self" {
            self.data_plane.send(self.instance_id, msg.to_string()).await;
            Ok(())
        } else if let Some(target) = self.callback_table.get_mapping(alias).await {
            self.data_plane.send(target, msg.to_string()).await;
            Ok(())
        } else {
            Err(GuestAPIError::UnknownAlias)
        }
    }

    pub async fn cast_raw(&mut self, target: edgeless_api::function_instance::InstanceId, msg: &str) -> Result<(), GuestAPIError> {
        self.data_plane.send(target, msg.to_string()).await;
        Ok(())
    }

    pub async fn call_alias(&mut self, alias: &str, msg: &str) -> Result<edgeless_dataplane::core::CallRet, GuestAPIError> {
        if alias == "self" {
            self.call_raw(self.instance_id, msg).await
            // return Ok(self.data_plane.call(self.instance_id.clone(), msg.to_string()).await);
        } else if let Some(target) = self.callback_table.get_mapping(alias).await {
            return self.call_raw(target, msg).await;
            // return Ok(self.data_plane.call(target.clone(), msg.to_string()).await);
        } else {
            log::warn!("Unknown alias {}.", alias);
            Err(GuestAPIError::UnknownAlias)
        }
    }

    pub async fn call_raw(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        msg: &str,
    ) -> Result<edgeless_dataplane::core::CallRet, GuestAPIError> {
        futures::select! {
            _ = Box::pin(self.poison_pill_receiver.recv()).fuse() => {
                Ok(edgeless_dataplane::core::CallRet::Err)
            },
            call_res = Box::pin(self.data_plane.call(target, msg.to_string())).fuse() => {
                Ok(call_res)
            }
        }
    }

    pub async fn telemetry_log(&mut self, lvl: edgeless_telemetry::telemetry_events::TelemetryLogLevel, target: &str, msg: &str) {
        println!("executing telemetry_log: {}", msg);
        self.telemetry_handle.observe(
            edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionLogEntry(lvl, target.to_string(), msg.to_string()),
            std::collections::BTreeMap::new(),
        );
    }

    pub async fn slf(&mut self) -> edgeless_api::function_instance::InstanceId {
        self.instance_id
    }

    pub async fn delayed_cast(&mut self, delay: u64, target_alias: &str, payload: &str) -> Result<(), GuestAPIError> {
        let mut cloned_plane = self.data_plane.clone();
        let cloned_msg = payload.to_string();

        let target_instance_id = if target_alias == "self" {
            self.instance_id
        } else if let Some(targted_id) = self.callback_table.get_mapping(target_alias).await {
            targted_id
        } else {
            log::warn!("Unknown alias {}.", target_alias);
            return Err(GuestAPIError::UnknownAlias);
        };

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            cloned_plane.send(target_instance_id, cloned_msg).await;
        });

        Ok(())
    }

    pub async fn sync(&mut self, serialized_state: &str) -> Result<(), GuestAPIError> {
        self.state_handle.set(serialized_state.to_string()).await;
        log::info!("Function State Sync: {}", serialized_state);
        Ok(())
    }
}
