# EDGELESS Cloud Offloading

**EDGELESS Cloud Offloading** is an EDGELESS component responsible of the creation of EDGELESS nodes in the cloud for EDGELESS clusters during periods of high demand. Built on top of the AWS SDK for Rust, this component provides a Delegated Orchestrator that detects when the cluster is saturated and automatically launches new EC2 instances to handle the load. It also manages the termination of underutilized nodes to optimize costs.

## Features

- Launch AWS EC2 instances with specified configurations. Automatically launches and configures new EC2 instances as EDGELESS nodes when the cluster shows signs of saturation. The decision is based on a logic that considers both the relative load imbalance between nodes (credit) and absolute resource saturation (CPU and Memory).
- Terminate EC2 instances. Safely drains and terminates underutilized nodes to save costs.

## Prerequisites

Before using the EDGELESS Cloud Offloading package, ensure you have the following:

- An AWS account and appropriate permissions to create and manage EC2 instances.
- An AWS EC2 Image (AMI) ID for the region you are working in, with EDGELESS installed.
- A security group that allows inbound traffic for the TCP ports used by EDGELESS (default ports in range 7000-7200).
- An EDGELESS orchestrator running and accessible from the internet.

## General Configurations

All the configuration properties are defined in a *cloud_offloading.toml* file, which is located in */target/debug*. You can generate a default configuration file and then edit it.

To generate the default configuration file, run the following command:

```bash
./cloud_offloading -t
```

The configuration file has the following structure:

```toml
[General]
# Interval in seconds between each cluster check cycle
CheckIntervalSeconds = 15

[Cluster]
# URL of the orchestrator's Redis proxy.
RedisUrl = "redis://127.0.0.1:6379"
# URL of the main orchestrator (used to configure new nodes)
OrchestratorUrl = "127.0.0.1"
# Minimum number of nodes the system will attempt to keep active
MinimumNodes = 1

[CloudProvider]
[CloudProvider.Aws]
# AWS region where instances will be created/deleted
Region = "eu-west-1"
# Amazon Machine Image (AMI) ID to be used for new nodes
AmiId = "ami-xxxxxxxxxxxxxxxxx"
# EC2 instance type for new nodes
InstanceType = "t2.medium"
# ID of the Security Group to be assigned to new instances
SecurityGroupId = "sg-xxxxxxxxxxxxxxxxx"

[Scaling]
[Scaling.Thresholds]
# --- Scale-Up Triggers ---
# Sum of "credits" from overloaded nodes to trigger a scale-up
CreditOverload = 1.0
# CPU percentage above which a node is considered saturated
CpuHighPercent = 80.0
# Memory percentage above which a node is considered saturated
MemHighPercent = 80.0

# --- Scale-Down Triggers ---
# CPU percentage below which a node is considered underutilized
CpuLowPercent = 10.0
# Memory percentage below which a node is considered underutilized
MemLowPercent = 20.0
# Time in seconds a node must remain underutilized before being deleted
DeleteCooldownSeconds = 300
```

## AWS Credentials Configuration

To interact with AWS services, you need to provide AWS credentials. Here are the recommended methods to configure these credentials securely:

### 1. AWS Credentials File

AWS CLI tools and SDKs look for credentials in the `~/.aws/credentials` file by default. You can set up your credentials like so:

1. Open or create the file `~/.aws/credentials`.
2. Add your credentials in the following format:

    ```ini
    [default]
    aws_access_key_id = YOUR_ACCESS_KEY
    aws_secret_access_key = YOUR_SECRET_KEY
    ```

### 2. Environment Variables

You can also configure your credentials using environment variables. Set these in your shell or system environment:

```bash
export AWS_ACCESS_KEY_ID=YOUR_ACCESS_KEY
export AWS_SECRET_ACCESS_KEY=YOUR_SECRET_KEY
```

## Launch Cloud Offloading component
Launch the executable. It is recommended to set the log level to *info* to see the decisions the component is making.

```bash
RUST_LOG=info ./cloud_offloading
```

