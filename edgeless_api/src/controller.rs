// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
pub trait ControllerAPI: Sync {
    fn workflow_instance_api(&mut self) -> Box<dyn crate::workflow_instance::WorkflowInstanceAPI>;
}
