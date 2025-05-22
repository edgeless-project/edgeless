// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::str::FromStr;

/// Intent to update/change deployment.
#[derive(Clone)]
pub enum DeployIntent {
    /// The component with givel logical identifier should be migrated to
    /// the given target nodes, if possible.
    Migrate(edgeless_api::function_instance::ComponentId, Vec<edgeless_api::function_instance::NodeId>),
}

impl DeployIntent {
    pub fn new(key: &str, value: &str) -> anyhow::Result<Self> {
        let tokens: Vec<&str> = key.split(':').collect();
        assert!(!tokens.is_empty());
        anyhow::ensure!(tokens[0] == "intent", "intent not starting with \"intent\"");
        if tokens.len() >= 2 {
            match tokens[1] {
                "migrate" => {
                    anyhow::ensure!(tokens.len() == 3);
                    let component_id = uuid::Uuid::from_str(tokens[2])?;
                    let mut targets = vec![];
                    for target in value.split(',') {
                        if target.is_empty() {
                            continue;
                        }
                        targets.push(uuid::Uuid::from_str(target)?);
                    }
                    Ok(DeployIntent::Migrate(component_id, targets))
                }
                _ => anyhow::bail!("unknown intent type '{}'", tokens[1]),
            }
        } else {
            anyhow::bail!("ill-formed intent");
        }
    }

    pub fn key(&self) -> String {
        match self {
            Self::Migrate(component, _) => format!("intent:migrate:{}", component),
        }
    }

    pub fn value(&self) -> String {
        match self {
            Self::Migrate(_, targets) => targets.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
        }
    }
}

impl std::fmt::Display for DeployIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DeployIntent::Migrate(component, target) => write!(
                f,
                "migrate component {} to [{}]",
                component,
                target.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")
            ),
        }
    }
}
