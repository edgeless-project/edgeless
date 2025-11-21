# DDA Evaluation

## Prerequisites

### 1. Create Python Virtual Environment

```bash
python -m venv .venv
source .venv/bin/activate
```

### 2. Install Python Dependencies

```bash
pip install pandas plotnine
```

### 3. Install System Dependencies

> MacOS specific, on other platforms please use the respective package manager.

```bash
# Install Go runtime
brew install go

# Install Redis
brew install redis
```

### 4. Install and Start Docker

Make sure Docker is installed and running on your system. You can download it from [docker.com](https://www.docker.com/products/docker-desktop/).

To verify Docker is running:

```bash
docker --version
docker info
```

## Running the Evaluation

### Step 0: Build the Project

```bash
cd scripts
./build.sh
```

### Step 1: Start DDA Service

```bash
docker run --name dda \
  -v ./dda_cfg/:/dda/ \
  -p 12000:12000 \
  --network host \
  ghcr.io/coatyio/dda:latest
```

### Step 2: Start MQTT Broker

```bash
docker run --name mqtt \
  -p 1883:1883 \
  --network host \
  eclipse-mosquitto:latest \
  mosquitto -c /mosquitto-no-auth.conf
```

### Step 3: Start DDA Endpoint

```bash
cd dda_endpoint
go run .
```

### Step 4: Start Redis Server

```bash
redis-server
```

Or using Docker:

```bash
docker run --name redis -p 6379:6379 --network host redis:latest
```

### Step 5: Start Edgeless Runtime

```bash
cd binaries
RUST_LOG=info ./edgeless_inabox
```

### Step 6: Run Benchmark

```bash
cd scripts
./bench.sh
```

### Step 7: Analyze Results

Open and explore the results in `visualize.ipynb`. Make sure to select a Python kernel >= 3.12.
