// SPDX-FileCopyrightText: © 2023 TUM
// SPDX-License-Identifier: MIT
pub trait ControllerAPI: Sync {
    fn workflow_instance_api(&mut self) -> Box<dyn crate::workflow_instance::WorkflowInstanceAPI>;
}
