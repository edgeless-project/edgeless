// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(PartialEq, Debug, Clone)]
pub enum AffinityLevel {
    Required,
    NotRequired,
}

impl std::fmt::Display for AffinityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AffinityLevel::Required => "required",
                AffinityLevel::NotRequired => "not-required",
            }
        )
    }
}

impl AffinityLevel {
    pub fn from_string(val: &str) -> Self {
        if val.to_lowercase() == "required" {
            AffinityLevel::Required
        } else {
            AffinityLevel::NotRequired
        }
    }
}
