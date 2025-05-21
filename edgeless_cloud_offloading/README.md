# EDGELESS Cloud Offloading

**EDGELESS Cloud Offloading** is an EDGELESS component responsible of the creation of EDGELESS nodes in the cloud for EDGELESS clusters. Built on top of the AWS SDK for Rust, this package provides a straightforward interface for managing cloud resources programmatically.

## Features

- Launch AWS EC2 instances with specified configurations.
- Configure EC2 instances as EDGELESS nodes.
- Terminate EC2 instances safely.

## Prerequisites

Before using the EDGELESS Cloud Offloading package, ensure you have the following:

- An AWS account and appropriate permissions to create and manage EC2 instances.
- An AWS EC2 Image (AMI) ID for the region you are working in, with EDGELESS installed.
- A security group that allows inbound traffic for the TCP ports used by EDGELESS (default ports in range 7000-7200).
- An EDGELESS orchestrator running and accessible from the internet.

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

## Configuration of AWS Region and other settings

To specify the AWS region for your operations and other configurations, you can use a `config.toml` file located in the root directory of this package.

This file should look like the following:

```toml
[aws]
region = "eu-west-1"
ami_id = "ami-035085b5449b0383a"
instance_type = "t2.micro"
security_group_id = "sg-09dcfc636643d2868"

[orchestrator]
url = "3.253.97.217"
```