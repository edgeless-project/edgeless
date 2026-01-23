#!/bin/bash

# Long-running KPI-13 experiment script
# Instead of killing nodes progressively (limited data points),
# this script cycles through nodes: kill -> wait -> restart -> wait -> repeat
# This generates continuous failover events for statistical analysis.

# Default values
# Key timing considerations:
# - subscription_refresh_interval_sec = 0.2 (from node configs) - orchestrator detects node loss in ~400ms
# - Actual failover is very fast (~50ms) once detected
# - Node re-registration takes ~0.5-1 seconds
KILL_INTERVAL_MIN=0.5   # Minimum time to wait after killing a node (randomized)
KILL_INTERVAL_MAX=1.0   # Maximum time to wait after killing a node (randomized)
RESTART_INTERVAL=2      # Time to wait after restarting a node (for re-registration)
TOTAL_DURATION=300      # Total experiment duration in seconds (5 minutes default)
RUST_LOG_LEVEL="info"
STARTUP_DELAY=2
MIN_NODES=2             # Minimum nodes to keep alive (need at least 2 for replication)
BATCH_SIZE=1            # Number of nodes to kill at once (more = more failovers per cycle)
QUIET_FUNCTIONS=false   # Silence WASM function output
REPEAT_COUNT=1          # Number of times to repeat the experiment
STRESS_NG=false         # Whether to run stress-ng during the experiment
STRESS_NG_ARGS="--cpu $(nproc) --cpu-method matrixprod"  # Default stress-ng arguments

show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

Long-running Edgeless failover experiment that cycles through killing and
restarting edge nodes to generate many failover data points.

HOW IT WORKS:
    1. Starts all infrastructure (controller, orchestrator, 9 edge nodes)
    2. Starts the workflow (work_splitter + 9 calculators with replication_factor=2)
    3. Cycles through nodes: kill one -> wait for failover -> restart it -> wait
    4. Continues until TOTAL_DURATION is reached
    5. Each node kill triggers:
       - del_node span (node removal)
       - refresh span (orchestrator refresh cycle)
       - kpi_13_failover span (hot-standby promotion + replica recreation)
       - apply_patches span (dataplane repatching)

OPTIONS:
    -d, --duration SEC        Total experiment duration in seconds (default: $TOTAL_DURATION)
    -k, --kill-interval SEC   Wait time after killing node(s) - randomized between 0.5-1.0s by default
    -r, --restart-interval SEC Wait time after restarting a node (default: $RESTART_INTERVAL)
    -b, --batch-size NUM      Number of nodes to kill at once (default: $BATCH_SIZE)
    -m, --min-nodes NUM       Minimum nodes to keep alive (default: $MIN_NODES)
    -s, --startup-delay SEC   Delay after starting orchestrator (default: $STARTUP_DELAY)
    -l, --log-level LEVEL     Rust log level: error, warn, info, debug, trace (default: $RUST_LOG_LEVEL)
    -n, --repeat NUM          Number of times to repeat the experiment (default: $REPEAT_COUNT)
    -q, --quiet-functions     Silence WASM function output for cleaner experiment logs
    --stress                  Run stress-ng for CPU/IO/memory load during the experiment
    --stress-args ARGS        Custom stress-ng arguments (default: "$STRESS_NG_ARGS")
    -h, --help                Show this help message

EXAMPLES:
    $0                              Run 5x 5 minutes with defaults
    $0 -n 1 -d 600                  Single run for 10 minutes
    $0 -d 300 -b 3                  5x 5 min, kill 3 nodes at once
    $0 -d 1800 -k 3 -r 2 -n 3      3x 30 minutes
    $0 --stress                     5x 5 min with CPU/IO/memory stress
    $0 --stress --stress-args "--cpu 4 --vm 2 --vm-bytes 512M"  Custom stress load

TIMING NOTES:
    - Node subscription refresh interval is 50ms (orchestrator detection)
    - Actual failover is very fast (~50ms) once detected
    - Kill interval is randomized (0.5-1.0s) to avoid periodic patterns
    - Minimum practical cycle time is ~1.5-2 seconds (0.5-1s kill + 1s restart)
    - Batch killing multiple nodes triggers multiple failovers in one cycle!

ESTIMATED DATA POINTS:
    With default settings (kill=0.5-1.0s avg 0.75s, restart=1s, batch=1, duration=300s):
    ~300/1.75 ≈ 170 failover events
    
    With batch=3: ~300/2 * 3 ≈ 450 failover events
    For 100+ data points, use: $0 -d 200 or $0 -d 70 -b 3
    For 300+ data points, use: $0 -d 600 or $0 -d 200 -b 3

MONITORING:
    Results are stored in: ../results/orchestrator_events_long_*.csv
    Monitor in real-time: tail -f ../results/orchestrator_events_long_*.csv

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--duration)
            TOTAL_DURATION="$2"
            shift 2
            ;;
        -k|--kill-interval)
            KILL_INTERVAL_MIN="$2"
            KILL_INTERVAL_MAX="$2"
            shift 2
            ;;
        -r|--restart-interval)
            RESTART_INTERVAL="$2"
            shift 2
            ;;
        -m|--min-nodes)
            MIN_NODES="$2"
            shift 2
            ;;
        -b|--batch-size)
            BATCH_SIZE="$2"
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
        -n|--repeat)
            REPEAT_COUNT="$2"
            shift 2
            ;;
        -q|--quiet-functions)
            QUIET_FUNCTIONS=true
            shift
            ;;
        --stress)
            STRESS_NG=true
            shift
            ;;
        --stress-args)
            STRESS_NG_ARGS="$2"
            STRESS_NG=true
            shift 2
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
if ! [[ "$TOTAL_DURATION" =~ ^[0-9]+$ ]] || [ "$TOTAL_DURATION" -lt 30 ]; then
    echo "Error: --duration must be at least 30 seconds"
    exit 1
fi

if ! [[ "$RESTART_INTERVAL" =~ ^[0-9]+$ ]] || [ "$RESTART_INTERVAL" -lt 1 ]; then
    echo "Error: --restart-interval must be a positive integer"
    exit 1
fi

if ! [[ "$MIN_NODES" =~ ^[0-9]+$ ]] || [ "$MIN_NODES" -lt 1 ]; then
    echo "Error: --min-nodes must be at least 1"
    exit 1
fi

if ! [[ "$BATCH_SIZE" =~ ^[0-9]+$ ]] || [ "$BATCH_SIZE" -lt 1 ]; then
    echo "Error: --batch-size must be at least 1"
    exit 1
fi

case "$RUST_LOG_LEVEL" in
    error|warn|info|debug|trace) ;;
    *) echo "Error: --log-level must be one of: error, warn, info, debug, trace"; exit 1 ;;
esac

if ! [[ "$REPEAT_COUNT" =~ ^[0-9]+$ ]] || [ "$REPEAT_COUNT" -lt 1 ]; then
    echo "Error: --repeat must be a positive integer"
    exit 1
fi

if [ "$STRESS_NG" = true ] && ! command -v stress-ng &> /dev/null; then
    echo "Error: stress-ng is not installed. Install with: sudo apt install stress-ng"
    exit 1
fi

# Global variables
controller_pid=""
orchestrator_pid=""
declare -A node_pids         # Associative array: config_name -> pid
redis_pid=""
redis_started_by_script=false
stress_ng_pid=""
experiment_start_time=0
failover_count=0
total_failover_count=0

# Cleanup function
cleanup() {
    echo
    echo "=== Received interrupt signal, cleaning up... ==="
    
    # Kill all edge nodes
    for cfg in "${!node_pids[@]}"; do
        pid="${node_pids[$cfg]}"
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            echo "Stopping edge node $cfg (PID $pid)"
            kill "$pid" 2>/dev/null || true
        fi
    done
    
    # Kill controller
    if [ -n "$controller_pid" ] && kill -0 "$controller_pid" 2>/dev/null; then
        echo "Stopping controller PID $controller_pid"
        kill "$controller_pid" 2>/dev/null || true
    fi
    
    # Kill orchestrator
    if [ -n "$orchestrator_pid" ] && kill -0 "$orchestrator_pid" 2>/dev/null; then
        echo "Stopping orchestrator PID $orchestrator_pid"
        kill "$orchestrator_pid" 2>/dev/null || true
    fi
    
    # Kill stress-ng if running
    if [ -n "$stress_ng_pid" ] && kill -0 "$stress_ng_pid" 2>/dev/null; then
        echo "Stopping stress-ng PID $stress_ng_pid"
        kill "$stress_ng_pid" 2>/dev/null || true
    fi
    
    # Kill Redis if we started it
    if [ "$redis_started_by_script" = true ] && [ -n "$redis_pid" ] && kill -0 "$redis_pid" 2>/dev/null; then
        echo "Stopping Redis server PID $redis_pid"
        kill "$redis_pid" 2>/dev/null || true
    fi
    
    echo "=== Cleanup completed ==="
    echo "Total failover events triggered: $((total_failover_count + failover_count))"
    exit 1
}

trap cleanup SIGINT SIGTERM

# Configuration - use relative paths from script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
CFG_DIR="$SCRIPT_DIR/../cfg"
RESULTS_DIR="$SCRIPT_DIR/../results"
BUILD_DIR="$SCRIPT_DIR/../build"

EDGE_NODE_BIN="$BUILD_DIR/edgeless_node_d"
CONTROLLER_BIN="$BUILD_DIR/edgeless_con_d"
ORCHESTRATOR_BIN="$BUILD_DIR/edgeless_orc_d"

CONTROLLER_CFG="controller.toml"
ORCHESTRATOR_CFG="orchestrator.toml"

# Node configurations - node0 is protected (runs work_splitter and is NEVER killed)
PROTECTED_NODE="node0.toml"
KILLABLE_NODES=(
    "node1.toml"
    "node2.toml"
    "node3.toml"
    "node4.toml"
    "node5.toml"
    "node6.toml"
    "node7.toml"
    "node8.toml"
)

mkdir -p "$RESULTS_DIR"

# Check/start Redis
if ! pgrep -x "redis-server" > /dev/null; then
    echo "Redis server not running, starting temporary instance..."
    redis-server --daemonize yes --port 6379
    sleep 2
    redis_pid=$(pgrep -x "redis-server" | head -1)
    redis_started_by_script=true
    echo "Redis server started with PID $redis_pid"
else
    echo "Redis server already running"
fi

# Calculate estimated data points (using average kill interval)
avg_kill_interval=$(awk -v min=$KILL_INTERVAL_MIN -v max=$KILL_INTERVAL_MAX 'BEGIN{printf "%.1f", (min+max)/2}')
cycle_time=$(awk -v kill=$avg_kill_interval -v restart=$RESTART_INTERVAL 'BEGIN{printf "%.1f", kill+restart}')
estimated_events=$(awk -v duration=$TOTAL_DURATION -v cycle=$cycle_time -v batch=$BATCH_SIZE 'BEGIN{printf "%d", (duration/cycle)*batch}')

# Construct RUST_LOG with optional function silencing
if [ "$QUIET_FUNCTIONS" = true ]; then
    RUST_LOG="$RUST_LOG_LEVEL,edgeless_telemetry::telemetry_events=off"
else
    RUST_LOG="$RUST_LOG_LEVEL"
fi

echo "=== Long-Running Experiment Configuration ==="
echo "Total duration: ${TOTAL_DURATION}s ($((TOTAL_DURATION / 60)) minutes) x $REPEAT_COUNT repeats"
echo "Kill interval: ${KILL_INTERVAL_MIN}s - ${KILL_INTERVAL_MAX}s (randomized)"
echo "Restart interval: ${RESTART_INTERVAL}s"
echo "Batch size: $BATCH_SIZE nodes per cycle"
echo "Minimum nodes: $MIN_NODES"
echo "Protected node: $PROTECTED_NODE (work_splitter - never killed)"
echo "Killable nodes: ${#KILLABLE_NODES[@]} (node1-node8)"
echo "Log level: $RUST_LOG_LEVEL"
echo "Quiet functions: $QUIET_FUNCTIONS"
echo "Repeat count: $REPEAT_COUNT"
echo "Stress-ng: $STRESS_NG"
if [ "$STRESS_NG" = true ]; then
    echo "Stress-ng args: $STRESS_NG_ARGS"
fi
echo "Estimated failover events per run: ~$estimated_events"
echo "Estimated total failover events: ~$((estimated_events * REPEAT_COUNT))"
echo "=============================================="

start_node() {
    local cfg="$1"
    echo "[$(date +%H:%M:%S)] Starting edge node: $cfg"
    RUST_LOG="$RUST_LOG" \
        "$EDGE_NODE_BIN" -c "$CFG_DIR/$cfg" 2>&1 &
    node_pids["$cfg"]=$!
    echo "[$(date +%H:%M:%S)] Node $cfg started with PID ${node_pids[$cfg]}"
}

stop_node() {
    local cfg="$1"
    local pid="${node_pids[$cfg]}"
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        echo "[$(date +%H:%M:%S)] Killing node $cfg (PID $pid) - triggering failover"
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true
        unset node_pids["$cfg"]
        ((failover_count++))
        echo "[$(date +%H:%M:%S)] Failover event #$failover_count triggered"
        return 0
    fi
    return 1
}

is_node_running() {
    local cfg="$1"
    local pid="${node_pids[$cfg]}"
    [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null
}

count_running_killable_nodes() {
    local count=0
    # Only count killable nodes (node1-node8), not the protected node (node0)
    for cfg in "${KILLABLE_NODES[@]}"; do
        if is_node_running "$cfg"; then
            ((count++))
        fi
    done
    echo "$count"
}

get_running_killable_nodes() {
    local running=()
    # Only return killable nodes (node1-node8), never the protected node (node0)
    for cfg in "${KILLABLE_NODES[@]}"; do
        if is_node_running "$cfg"; then
            running+=("$cfg")
        fi
    done
    echo "${running[@]}"
}

get_stopped_killable_nodes() {
    local stopped=()
    for cfg in "${KILLABLE_NODES[@]}"; do
        if ! is_node_running "$cfg"; then
            stopped+=("$cfg")
        fi
    done
    echo "${stopped[@]}"
}

# Shuffle array in-place using Fisher-Yates algorithm
shuffle_array() {
    local i tmp size rand
    size=${#running_nodes[@]}
    for ((i = size - 1; i > 0; i--)); do
        rand=$((RANDOM % (i + 1)))
        tmp="${running_nodes[i]}"
        running_nodes[i]="${running_nodes[rand]}"
        running_nodes[rand]="$tmp"
    done
}

# Main repeat loop
all_csv_files=()

for ((run=1; run<=REPEAT_COUNT; run++)); do

echo
echo "################################################################"
echo "###  RUN $run / $REPEAT_COUNT"
echo "################################################################"
echo

# Reset per-run counters
failover_count=0
controller_pid=""
orchestrator_pid=""
stress_ng_pid=""
declare -A node_pids

# Start stress-ng if requested (per-run)
if [ "$STRESS_NG" = true ]; then
    echo
    echo "=== Starting stress-ng (Run $run/$REPEAT_COUNT) ==="
    echo "Command: stress-ng $STRESS_NG_ARGS --timeout ${TOTAL_DURATION}s"
    stress-ng $STRESS_NG_ARGS --timeout "${TOTAL_DURATION}s" &
    stress_ng_pid=$!
    echo "stress-ng started with PID $stress_ng_pid"
    sleep 2
fi

# Start infrastructure
echo
echo "=== Starting Infrastructure (Run $run/$REPEAT_COUNT) ==="

# Start controller
echo "Starting controller"
RUST_LOG="$RUST_LOG" \
    "$CONTROLLER_BIN" -c "$CFG_DIR/$CONTROLLER_CFG" 2>&1 &
controller_pid=$!
echo "Controller started with PID $controller_pid"

sleep "$STARTUP_DELAY"

# Start orchestrator
echo "Starting orchestrator"
RUST_LOG="$RUST_LOG" \
    "$ORCHESTRATOR_BIN" -c "$CFG_DIR/$ORCHESTRATOR_CFG" 2>&1 &
orchestrator_pid=$!
echo "Orchestrator started with PID $orchestrator_pid"

sleep "$STARTUP_DELAY"

# Start protected node (node0 - runs work_splitter)
start_node "$PROTECTED_NODE"

# Start all killable nodes
for cfg in "${KILLABLE_NODES[@]}"; do
    start_node "$cfg"
    sleep 0.5  # Small delay between node starts
done

echo
echo "=== All nodes started, waiting for registration ==="
sleep 5

# Start workflow
echo
echo "=== Starting Workflow ==="
"$SCRIPT_DIR/start_workflow.sh"
echo "Workflow started, waiting for full initialization..."
sleep 10

# Create unique CSV filename
timestamp=$(date +"%Y%m%d_%H%M%S")
csv_filename="$RESULTS_DIR/orchestrator_events_long_run${run}_${timestamp}"

echo
echo "=== Beginning Failover Cycling (Run $run/$REPEAT_COUNT) ==="
echo "Results will be saved to: $csv_filename.csv"
echo "Monitor with: tail -f $csv_filename.csv"
echo

experiment_start_time=$(date +%s)
current_kill_index=0

while true; do
    current_time=$(date +%s)
    elapsed=$((current_time - experiment_start_time))
    remaining=$((TOTAL_DURATION - elapsed))
    
    if [ "$elapsed" -ge "$TOTAL_DURATION" ]; then
        echo
        echo "[$(date +%H:%M:%S)] Experiment duration reached ($TOTAL_DURATION seconds)"
        break
    fi
    
    echo "[$(date +%H:%M:%S)] Run $run/$REPEAT_COUNT | Elapsed: ${elapsed}s / ${TOTAL_DURATION}s | Failovers: $failover_count | Remaining: ${remaining}s"
    
    running_count=$(count_running_killable_nodes)
    
    # Strategy: Kill BATCH_SIZE nodes if we have enough running, then restart stopped nodes
    nodes_to_kill_count=$((running_count - MIN_NODES))
    if [ "$nodes_to_kill_count" -gt 0 ]; then
        # Limit to batch size
        if [ "$nodes_to_kill_count" -gt "$BATCH_SIZE" ]; then
            nodes_to_kill_count=$BATCH_SIZE
        fi
        
        # Get list of running killable nodes (excludes node0/work_splitter)
        running_nodes=($(get_running_killable_nodes))
        
        # Shuffle for random selection (only shuffles killable nodes, node0 is not in this array)
        shuffle_array
        
        # Kill batch of nodes (randomly selected)
        killed_this_cycle=0
        for ((i=0; i<nodes_to_kill_count && i<${#running_nodes[@]}; i++)); do
            node_to_kill="${running_nodes[i]}"
            
            stop_node "$node_to_kill"
            ((killed_this_cycle++))
        done
        
        if [ "$killed_this_cycle" -gt 0 ]; then
            # Randomize kill interval between min and max
            kill_interval=$(awk -v min=$KILL_INTERVAL_MIN -v max=$KILL_INTERVAL_MAX 'BEGIN{srand(); printf "%.3f", min+rand()*(max-min)}')
            echo "[$(date +%H:%M:%S)] Killed $killed_this_cycle node(s), waiting ${kill_interval}s for failover..."
            sleep "$kill_interval"
        fi
    else
        echo "[$(date +%H:%M:%S)] Only $running_count nodes running (min: $MIN_NODES), restarting stopped nodes..."
    fi
    
    # Restart any stopped nodes (to keep the cycle going)
    stopped_nodes=($(get_stopped_killable_nodes))
    for cfg in "${stopped_nodes[@]}"; do
        start_node "$cfg"
        echo "[$(date +%H:%M:%S)] Waiting ${RESTART_INTERVAL}s for node registration..."
        sleep "$RESTART_INTERVAL"
        
        # Check if we've exceeded duration during restart wait
        current_time=$(date +%s)
        elapsed=$((current_time - experiment_start_time))
        if [ "$elapsed" -ge "$TOTAL_DURATION" ]; then
            break 2
        fi
    done
done

echo
echo "=== Run $run/$REPEAT_COUNT Complete ==="
echo "Failover events this run: $failover_count"
echo "Duration: ${TOTAL_DURATION}s"

# Shutdown infrastructure for this run
echo
echo "=== Shutting down infrastructure (Run $run/$REPEAT_COUNT) ==="

# Kill all killable nodes
for cfg in "${KILLABLE_NODES[@]}"; do
    pid="${node_pids[$cfg]}"
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null || true
    fi
done

# Kill protected node
pid="${node_pids[$PROTECTED_NODE]}"
if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
fi

kill "$controller_pid" 2>/dev/null || true
kill "$orchestrator_pid" 2>/dev/null || true

# Wait briefly for processes to exit, then force-kill any stragglers
sleep 2
for cfg in "${!node_pids[@]}"; do
    pid="${node_pids[$cfg]}"
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
        echo "Force-killing node $cfg (PID $pid)"
        kill -9 "$pid" 2>/dev/null || true
    fi
done
if [ -n "$controller_pid" ] && kill -0 "$controller_pid" 2>/dev/null; then
    echo "Force-killing controller (PID $controller_pid)"
    kill -9 "$controller_pid" 2>/dev/null || true
fi
if [ -n "$orchestrator_pid" ] && kill -0 "$orchestrator_pid" 2>/dev/null; then
    echo "Force-killing orchestrator (PID $orchestrator_pid)"
    kill -9 "$orchestrator_pid" 2>/dev/null || true
fi

# Move results
if [ -f "/tmp/orchestrator_kpi_samples.csv" ]; then
    mv "/tmp/orchestrator_kpi_samples.csv" "$csv_filename.csv"
    echo "Results saved to: $csv_filename.csv"
    all_csv_files+=("$csv_filename.csv")
    
    # Quick stats
    if command -v wc &> /dev/null; then
        lines=$(wc -l < "$csv_filename.csv")
        echo "Total telemetry events recorded: $lines"
    fi
else
    echo "Warning: No telemetry file found at /tmp/orchestrator_kpi_samples.csv"
fi

# Stop stress-ng for this run
if [ -n "$stress_ng_pid" ] && kill -0 "$stress_ng_pid" 2>/dev/null; then
    echo "Stopping stress-ng (PID $stress_ng_pid)"
    kill "$stress_ng_pid" 2>/dev/null || true
    sleep 1
    if kill -0 "$stress_ng_pid" 2>/dev/null; then
        kill -9 "$stress_ng_pid" 2>/dev/null || true
    fi
fi

total_failover_count=$((total_failover_count + failover_count))

# Brief pause between runs
if [ "$run" -lt "$REPEAT_COUNT" ]; then
    echo
    echo "=== Pausing 5s before next run ==="
    sleep 5
fi

done  # end repeat loop

# Stop Redis if we started it
if [ "$redis_started_by_script" = true ] && [ -n "$redis_pid" ] && kill -0 "$redis_pid" 2>/dev/null; then
    echo "Stopping Redis server"
    kill "$redis_pid" 2>/dev/null || true
fi

trap - SIGINT SIGTERM

echo
echo "=== Final Summary ==="
echo "Runs completed: $REPEAT_COUNT"
echo "Total failover events triggered: $total_failover_count"
echo "Expected spans per failover: del_node, refresh, kpi_13_failover, apply_patches"
echo "Estimated total spans: ~$((total_failover_count * 4))"
if [ "$STRESS_NG" = true ]; then
    echo "Stress-ng was active: $STRESS_NG_ARGS"
fi
echo
echo "Result files:"
for f in "${all_csv_files[@]}"; do
    echo "  $f"
done
echo
echo "Analyze results with: jupyter notebook ../evaluate.ipynb"
