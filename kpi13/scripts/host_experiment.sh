#!/bin/bash

# Default values
KILL_INTERVAL=5
RUNS=1
RUST_LOG_LEVEL="info"
STARTUP_DELAY=2
QUIET_FUNCTIONS=false

show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

Edgeless failover experiment script that progressively kills edge nodes
to test orchestrator behavior and function migration.

OPTIONS:
    -r, --runs RUNS           Number of experiment runs to execute (default: $RUNS)
    -k, --kill-interval SEC   Interval in seconds between node kills (default: $KILL_INTERVAL)
    -s, --startup-delay SEC   Delay in seconds after starting orchestrator (default: $STARTUP_DELAY)
    -l, --log-level LEVEL     Rust log level: error, warn, info, debug, trace (default: $RUST_LOG_LEVEL)
    -q, --quiet-functions     Silence WASM function output for cleaner experiment logs
    -h, --help                Show this help message

EXAMPLES:
    $0                        Run once with defaults
    $0 -r 5                   Run 5 times
    $0 -r 3 -k 10 -l debug    Run 3 times, kill every 10s, debug logging
    
MONITORING:
    Results are stored in ../results/orchestrator_events_run*.csv
    You can monitor in real-time with: tail -f ../results/orchestrator_events_run*.csv

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -r|--runs)
            RUNS="$2"
            shift 2
            ;;
        -k|--kill-interval)
            KILL_INTERVAL="$2"
            shift 2
            ;;
        -s|--startup-delay)
            STARTUP_DELAY="$2"
            shift 2
            ;;
        -l|--log-level)
            RUST_LOG_LEVEL="$2"
            shift 2
            ;;
        -q|--quiet-functions)
            QUIET_FUNCTIONS=true
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Validate arguments
if ! [[ "$RUNS" =~ ^[0-9]+$ ]] || [ "$RUNS" -lt 1 ]; then
    echo "Error: --runs must be a positive integer"
    exit 1
fi

if ! [[ "$KILL_INTERVAL" =~ ^[0-9]+$ ]] || [ "$KILL_INTERVAL" -lt 1 ]; then
    echo "Error: --kill-interval must be a positive integer"
    exit 1
fi

if ! [[ "$STARTUP_DELAY" =~ ^[0-9]+$ ]] || [ "$STARTUP_DELAY" -lt 0 ]; then
    echo "Error: --startup-delay must be a non-negative integer"
    exit 1
fi

case "$RUST_LOG_LEVEL" in
    error|warn|info|debug|trace) ;;
    *) echo "Error: --log-level must be one of: error, warn, info, debug, trace"; exit 1 ;;
esac

# Global variables to track running processes
controller_pid=""
orchestrator_pid=""
edge_pids=()
redis_pid=""
redis_started_by_script=false

# Cleanup function for Ctrl-C
cleanup() {
    echo
    echo "=== Received interrupt signal, cleaning up... ==="
    
    # Kill edge nodes
    for pid in "${edge_pids[@]}"; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            echo "Stopping edge node PID $pid"
            kill "$pid" || true
        fi
    done
    
    # Kill controller
    if [ -n "$controller_pid" ] && kill -0 "$controller_pid" 2>/dev/null; then
        echo "Stopping controller PID $controller_pid"
        kill "$controller_pid" || true
    fi
    
    # Kill orchestrator
    if [ -n "$orchestrator_pid" ] && kill -0 "$orchestrator_pid" 2>/dev/null; then
        echo "Stopping orchestrator PID $orchestrator_pid"
        kill "$orchestrator_pid" || true
    fi
    
    # Kill Redis if we started it
    if [ "$redis_started_by_script" = true ] && [ -n "$redis_pid" ] && kill -0 "$redis_pid" 2>/dev/null; then
        echo "Stopping Redis server PID $redis_pid"
        kill "$redis_pid" || true
    fi
    
    echo "=== Cleanup completed ==="
    exit 1
}

# Set up signal handler for Ctrl-C
trap cleanup SIGINT SIGTERM

# Configuration - use relative paths from script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
CFG_DIR="$SCRIPT_DIR/../cfg/host"
RESULTS_DIR="$SCRIPT_DIR/../results"

# Create results directory if it doesn't exist
mkdir -p "$RESULTS_DIR"

# Check if Redis is running, start it if needed
if ! pgrep -x "redis-server" > /dev/null; then
    echo "Redis server not running, starting temporary instance..."
    redis-server --daemonize yes --port 6379 &
    redis_pid=$!
    redis_started_by_script=true
    # Wait a moment for Redis to start
    sleep 2
    echo "Redis server started with PID $redis_pid"
else
    echo "Redis server already running"
fi

BUILD_DIR="$SCRIPT_DIR/../build"
EDGE_NODE_BIN="$BUILD_DIR/edgeless_node_d"
EDGE_NODE_CFGS=(
  node0.toml
  node1.toml
  node2.toml
  node3.toml
  node4.toml
  node5.toml
  node6.toml
  node7.toml
  node8.toml
)
CONTROLLER_BIN="$BUILD_DIR/edgeless_con_d"
CONTROLLER_CFG="controller.toml"
ORCHESTRATOR_BIN="$BUILD_DIR/edgeless_orc_d"
ORCHESTRATOR_CFG="orchestrator.toml"

# Construct RUST_LOG with optional function silencing
if [ "$QUIET_FUNCTIONS" = true ]; then
    RUST_LOG="$RUST_LOG_LEVEL,edgeless_telemetry::telemetry_events=off"
else
    RUST_LOG="$RUST_LOG_LEVEL"
fi

echo "=== Experiment Configuration ==="
echo "Runs: $RUNS"
echo "Kill interval: ${KILL_INTERVAL}s"
echo "Startup delay: ${STARTUP_DELAY}s"
echo "Log level: $RUST_LOG_LEVEL"
echo "Quiet functions: $QUIET_FUNCTIONS"
echo "==============================="

run_once() {
    echo "=== Starting new run ==="

    # Start controller
    echo "Starting controller"
    RUST_LOG="$RUST_LOG" \
        $CONTROLLER_BIN -c "$CFG_DIR/$CONTROLLER_CFG" 2>&1 &

    controller_pid=$!

    # Start orchestrator
    sleep "$STARTUP_DELAY"
    echo "Starting orchestrator"
    RUST_LOG="$RUST_LOG" \
        $ORCHESTRATOR_BIN -c "$CFG_DIR/$ORCHESTRATOR_CFG" 2>&1 &

    orchestrator_pid=$!

    # Start edge nodes
    edge_pids=()
    node0_pid=""
    killable_pids=()
    
    for i in "${!EDGE_NODE_CFGS[@]}"; do
        cfg="${EDGE_NODE_CFGS[$i]}"
        echo "Starting edge node with config $cfg"
        RUST_LOG="$RUST_LOG" \
            $EDGE_NODE_BIN -c "$CFG_DIR/$cfg" 2>&1 &
        pid=$!
        edge_pids+=($pid)
        
        # Keep node0.toml separate - it should never be killed
        if [ "$cfg" = "node0.toml" ]; then
            node0_pid=$pid
            echo "Node0 (protected) PID: $pid"
        else
            killable_pids+=($pid)
        fi
    done

    echo "All components started"
    echo "Edge node PIDs: ${edge_pids[*]}"
    echo "Killable node PIDs: ${killable_pids[*]}"

    # start the workflow here and then wait
    sleep 5 # wait a bit before starting the workflow
    echo "Starting workflow"
    "$SCRIPT_DIR/start_workflow.sh"
    sleep 5 # wait until the workflow is fully started

    # Progressive teardown of edge nodes (except node0)
    while ((${#killable_pids[@]} > 1)); do
        sleep "$KILL_INTERVAL"

        pid_to_kill="${killable_pids[0]}"
        echo "Stopping edge node PID $pid_to_kill (node0 protected)"
        kill "$pid_to_kill" || true

        killable_pids=("${killable_pids[@]:1}")
    done

    echo "One killable edge node remaining: ${killable_pids[0]} (plus protected node0: $node0_pid)"

    # Optional: let the final node run for a bit
    sleep 5

    echo "Stopping remaining components"

    # Kill remaining killable nodes and node0
    for pid in "${killable_pids[@]}"; do
        kill "$pid" || true
    done
    if [ -n "$node0_pid" ]; then
        kill "$node0_pid" || true
    fi
    
    kill "$controller_pid" || true
    kill "$orchestrator_pid" || true

    wait || true

    echo "=== Run finished ==="
    
    # Clear PIDs after successful completion
    controller_pid=""
    orchestrator_pid=""
    edge_pids=()
}

echo "You can tail orchestrator events with: tail -f $RESULTS_DIR/orchestrator_events_run*.csv"

# repeat the experiment RUNS times
for ((run=1; run<=RUNS; run++)); do
    echo
    echo "########## RUN $run / $RUNS ##########"
    
    # Create unique CSV filename for this run
    timestamp=$(date +"%Y%m%d_%H%M%S")
    csv_filename="$RESULTS_DIR/orchestrator_events_run${run}_${timestamp}"
    
    run_once

    # Move the orchestrator events CSV to the unique filename
    if [ -f "/tmp/orchestrator_kpi_samples.csv" ]; then
        mv "/tmp/orchestrator_kpi_samples.csv" "$csv_filename.csv"
    fi
    
    echo "Run $run results stored in: $csv_filename"
done

echo "All runs completed. Result files:"
ls -la "$RESULTS_DIR"/orchestrator_events_run*.csv 2>/dev/null || echo "No result files found"

# Restore original config if backup exists
if [ -f "$CFG_DIR/orchestrator.toml.bak" ]; then
    mv "$CFG_DIR/orchestrator.toml.bak" "$CFG_DIR/orchestrator.toml"
    echo "Restored original orchestrator.toml"
fi

# Stop Redis if we started it
if [ "$redis_started_by_script" = true ] && [ -n "$redis_pid" ] && kill -0 "$redis_pid" 2>/dev/null; then
    echo "Stopping Redis server PID $redis_pid"
    kill "$redis_pid" || true
    echo "Redis server stopped"
fi

# Clear the signal handler since we're done
trap - SIGINT SIGTERM
