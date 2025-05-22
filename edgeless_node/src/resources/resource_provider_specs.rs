// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub trait ResourceProviderSpecs {
    fn class_type(&self) -> String;
    fn outputs(&self) -> Vec<String>;
    fn configurations(&self) -> std::collections::HashMap<String, String>;
    fn version(&self) -> String;
}

#[derive(serde::Serialize)]
pub struct ResourceProviderSpecOutput {
    class_type: String,
    version: String,
    outputs: Vec<String>,
    configurations: std::collections::HashMap<String, String>,
}

impl dyn ResourceProviderSpecs {
    pub fn to_output(&self) -> ResourceProviderSpecOutput {
        ResourceProviderSpecOutput {
            class_type: self.class_type(),
            version: self.version(),
            outputs: self.outputs(),
            configurations: self.configurations(),
        }
    }
}
