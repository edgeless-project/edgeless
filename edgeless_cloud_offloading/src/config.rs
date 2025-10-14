use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub cluster: ClusterConfig,
    pub cloud_provider: CloudProviderConfig,
    pub scaling: ScalingConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GeneralConfig {
    pub check_interval_seconds: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClusterConfig {
    pub redis_url: String,
    pub orchestrator_url: String,
    pub minimum_nodes: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CloudProviderConfig {
    pub aws: AwsConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AwsConfig {
    pub region: String,
    pub ami_id: String,
    pub instance_type: String,
    pub security_group_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScalingConfig {
    pub thresholds: ThresholdsConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ThresholdsConfig {
    pub credit_overload: f64,
    pub cpu_high_percent: f64,
    pub mem_high_percent: f64,
    pub cpu_low_percent: f64,
    pub mem_low_percent: f64,
    pub delete_cooldown_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            cluster: ClusterConfig::default(),
            cloud_provider: CloudProviderConfig::default(),
            scaling: ScalingConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: 15,
        }
    }
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://127.0.0.1:6379".to_string(),
            orchestrator_url: "127.0.0.1".to_string(),
            minimum_nodes: 1,
        }
    }
}

impl Default for CloudProviderConfig {
    fn default() -> Self {
        Self {
            aws: AwsConfig::default(),
        }
    }
}

impl Default for AwsConfig {
    fn default() -> Self {
        Self {
            region: "eu-west-1".to_string(),
            ami_id: "ami-035085b5449b0383a".to_string(),
            instance_type: "t2.medium".to_string(),
            security_group_id: "sg-xxxxxxxxxxxxxxxxx".to_string(),
        }
    }
}

impl Default for ScalingConfig {
    fn default() -> Self {
        Self {
            thresholds: ThresholdsConfig::default(),
        }
    }
}

impl Default for ThresholdsConfig {
    fn default() -> Self {
        Self {
            credit_overload: 1.0,
            cpu_high_percent: 80.0,
            mem_high_percent: 80.0,
            cpu_low_percent: 30.0,
            mem_low_percent: 40.0,
            delete_cooldown_seconds: 30,
        }
    }
}
