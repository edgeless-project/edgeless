// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ResourceProvider {
    pub class_type: String,
    pub node_id: edgeless_api::function_instance::NodeId,
    pub outputs: Vec<String>,
}

impl std::fmt::Display for ResourceProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "class_type {}, node_id {}, outputs [{}]",
            self.class_type,
            self.node_id,
            self.outputs.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
        )
    }
}
