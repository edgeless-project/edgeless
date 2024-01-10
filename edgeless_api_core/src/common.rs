// SPDX-FileCopyrightText: © 2023 TUM
// SPDX-License-Identifier: MIT
#[derive(Clone, Debug)]
pub struct ErrorResponse {
    pub summary: &'static str,
    pub detail: Option<&'static str>,
}
