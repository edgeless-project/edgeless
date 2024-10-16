// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub trait ResourceProviderSpecs {
    fn class_type(&self) -> String;
    fn outputs(&self) -> Vec<String>;
    fn configurations(&self) -> std::collections::HashMap<String, String>;
    fn version(&self) -> String;
}
