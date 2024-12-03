// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, PartialEq)]
pub enum UpdatePeersRequest {
    Add(uuid::Uuid, String), // node_id, invocation_url
    Del(uuid::Uuid),         // node_id
    Clear,
}

#[async_trait::async_trait]
pub trait NodeManagementAPI: NodeManagementAPIClone + Sync + Send {
    async fn update_peers(&mut self, request: UpdatePeersRequest) -> anyhow::Result<()>;
}

// https://stackoverflow.com/a/30353928
pub trait NodeManagementAPIClone {
    fn clone_box(&self) -> Box<dyn NodeManagementAPI>;
}
impl<T> NodeManagementAPIClone for T
where
    T: 'static + NodeManagementAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn NodeManagementAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn NodeManagementAPI> {
    fn clone(&self) -> Box<dyn NodeManagementAPI> {
        self.clone_box()
    }
}
