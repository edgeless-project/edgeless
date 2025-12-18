#!/bin/bash

# User data script for EC2 instance (command script to run when the instance is launched)

# The edgeless-node image is based on a Ubuntu Server 24.04 LTS image
# The edgeless directory is: /home/ubuntu/edgeless
EDGELESS_DIR=/home/ubuntu/edgeless

# Orchestrator URL (Without http://)
ORCHESTRATOR_URL=__ORCHESTRATOR_URL_PLACEHOLDER__

# node Id
NODE_ID=__NODE_ID_PLACEHOLDER__

# Obtain EC2 instance Public IPv4 DNS (NODE_URL)

# Function to get a session token for IMDSv2
get_imds_token() {
    TOKEN=$(curl -s -X PUT "http://169.254.169.254/latest/api/token" -H "X-aws-ec2-metadata-token-ttl-seconds: 21600")
    if [ -z "$TOKEN" ]; then
        echo "Error: Failed to retrieve IMDSv2 token." >&2
        return 1
    fi
    echo "$TOKEN"
    return 0
}

# Function to fetch EC2 instance metadata using IMDSv2
get_ec2_metadata_v2() {
    local metadata_path="$1"
    local token="$2"
    local metadata_url="http://169.254.169.254/latest/meta-data/${metadata_path}"
    local value

    value=$(curl --fail --silent --connect-timeout 5 -H "X-aws-ec2-metadata-token: $token" "${metadata_url}")

    if [ $? -eq 0 ] && [ -n "$value" ]; then
        echo "$value"
        return 0
    else
        # Check if curl failed specifically because the path doesn't exist (exit code 22 for 404)
        # or other curl error
        local curl_exit_code=$?
        if [ $curl_exit_code -eq 22 ]; then # HTTP 404 Not Found
            echo "Error: Metadata path '${metadata_path}' not found (404)." >&2
        elif [ -z "$value" ]; then
            echo "Error: No value returned for metadata path '${metadata_path}' (is it empty?)." >&2
        else
            echo "Error: Failed to retrieve metadata for '${metadata_path}'. Curl exit code: $curl_exit_code" >&2
        fi
        return 1
    fi
}

# Attempt to get IMDSv2 token
IMDS_TOKEN=$(get_imds_token)
if [ $? -ne 0 ]; then
    echo "Error getting IMDSv2 token" >&2
    exit 1
fi
echo "Token IMDSv2 obtained"

# Get the public IPv4 DNS of the instance using IMDSv2
NODE_URL=$(get_ec2_metadata_v2 public-hostname "$IMDS_TOKEN")
if [ -z "$NODE_URL" ]; then
    echo "Error: Failed to retrieve public-hostname metadata." >&2
    exit 1
fi

# Get the instance ID using IMDSv2
INSTANCE_ID=$(get_ec2_metadata_v2 instance-id "$IMDS_TOKEN")
if [ -z "$INSTANCE_ID" ]; then
    echo "Error: Failed to retrieve public-hostname metadata." >&2
    exit 1
fi


# Create the default configuration of the node
$EDGELESS_DIR/target/debug/edgeless_node_d -t $EDGELESS_DIR/target/debug/node.toml

# Update the node.toml configuration file
sed -i.bak \
    -e "s#^\\(\\s*node_id\\s*=\\s*\\\"\\)[^\\\"]*\\(\\\".*\\)#\\1${NODE_ID}\\2#" \
    -e "s#^\\(\\s*node_register_url\\s*=\\s*\\\"\\)[^\\\"]*\\(\\\".*\\)#\\1http://${ORCHESTRATOR_URL}:7004\\2#" \
    -e 's#^\(\s*agent_url\s*=\s*"\)[^"]*\(".*\)#\1http://0.0.0.0:7005\2#' \
    -e "s#^\\(\\s*agent_url_announced\\s*=\\s*\\\"\\)[^\\\"]*\\(\\\".*\\)#\\1http://${NODE_URL}:7005\\2#" \
    -e 's#^\(\s*invocation_url\s*=\s*"\)[^"]*\(".*\)#\1http://0.0.0.0:7006\2#' \
    -e "s#^\\(\\s*invocation_url_announced\\s*=\\s*\\\"\\)[^\\\"]*\\(\\\".*\\)#\\1http://${NODE_URL}:7006\\2#" \
    -e "s#^\\(\\s*http_ingress_provider\\s*=\\s*\\\"\\)[^\\\"]*\\(\\\".*\\)#\\1http-ingress-${INSTANCE_ID}\\2#" \
    -e "s#^\\(\\s*http_egress_provider\\s*=\\s*\\\"\\)[^\\\"]*\\(\\\".*\\)#\\1http-egress-${INSTANCE_ID}\\2#" \
    -e "s#^\\(\\s*file_log_provider\\s*=\\s*\\\"\\)[^\\\"]*\\(\\\".*\\)#\\1file-log-${INSTANCE_ID}\\2#" \
    -e "s#^\\(\\s*redis_provider\\s*=\\s*\\\"\\)[^\\\"]*\\(\\\".*\\)#\\1redis-${INSTANCE_ID}\\2#" \
    -e "s#^\\(\\s*dda_provider\\s*=\\s*\\\"\\)[^\\\"]*\\(\\\".*\\)#\\1dda-${INSTANCE_ID}\\2#" \
    -e "s#^\\(\\s*sqlx_provider\\s*=\\s*\\\"\\)[^\\\"]*\\(\\\".*\\)#\\1sqlite-${INSTANCE_ID}\\2#" \
    "$EDGELESS_DIR/target/debug/node.toml"

# Start the node (it does not work without being in the directory)
cd $EDGELESS_DIR/target/debug
nohup env RUST_LOG=info ./edgeless_node_d > /home/ubuntu/edgeless_node_logs.txt 2>&1 &