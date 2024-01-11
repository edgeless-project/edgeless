// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#[derive(Clone, Debug)]
pub struct ErrorResponse {
    pub summary: &'static str,
    pub detail: Option<&'static str>,
}
