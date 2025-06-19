#!/usr/bin/env python3

import os
import argparse
import logging
import os.path

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Global Configuration (can be overridden by environment variables and command-line flags)
MONITOR_INTERVAL = int(os.environ.get("MONITOR_INTERVAL", 2))
PROXY_HOST = os.environ.get("PROXY_HOST", "127.0.0.1")
PROXY_PORT = int(os.environ.get("PROXY_PORT", 6379))
PROXY_DB = int(os.environ.get("PROXY_DB", 0))
TIME_WINDOW = int(os.environ.get("TIME_WINDOW", 1800))   # metrics in the last 30 minutes
MONITOR_NODE_HEALTH = os.environ.get("MONITOR_NODE_HEALTH", "true").lower() == "true"
MONITOR_FUNCTION_PERFORMANCE = os.environ.get("MONITOR_FUNCTION_PERFORMANCE", "true").lower() == "true"
CLEAN_CLI = os.environ.get("CLEAN_CLI", "true").lower() == "true"

# Output Configuration
OUTPUT_WRITE_TO_CSV = os.environ.get("OUTPUT_WRITE_TO_CSV", "true").lower() == "true"
OUTPUT_EXPERIMENT_NAME = os.environ.get("OUTPUT_EXPERIMENT_NAME", "mapreduce_flat")
OUTPUT_COLUMNS = os.environ.get("OUTPUT_COLUMNS", "true").lower() == "true"

# Model Configuration
DEFAULT_MODEL_PATH = os.environ.get(
    "MODEL_PATH", 
    os.path.join("outputs", "models", "random_model.pkl")
)

def validate_model_path(path):
    """Validate that the model file exists and has .pkl extension."""
    if not path.endswith('.pkl'):
        raise argparse.ArgumentTypeError("Model file must have .pkl extension")
    if not os.path.isfile(path):
        raise argparse.ArgumentTypeError(f"Model file not found: {path}")
    return path

def get_config():
    """
    Parse command line arguments and return configuration dictionary.
    
    Returns:
    --------
    dict
        Configuration dictionary with all settings
    """
    parser = argparse.ArgumentParser(description="EDGELESS Redis Metrics Subscriber")
    
    # Connection settings
    parser.add_argument('--proxy-host', type=str, default=PROXY_HOST, 
                       help=f'Redis host (default: {PROXY_HOST})')
    parser.add_argument('--proxy-port', type=int, default=PROXY_PORT, 
                       help=f'Redis port (default: {PROXY_PORT})')
    parser.add_argument('--proxy-db', type=int, default=PROXY_DB, 
                       help=f'Redis database (default: {PROXY_DB})')
    
    # Monitoring settings
    parser.add_argument('--interval', type=int, default=MONITOR_INTERVAL, 
                       help=f'Monitoring interval in seconds (default: {MONITOR_INTERVAL})')
    parser.add_argument('--time-window', type=int, default=TIME_WINDOW, 
                       help=f'Time window in seconds for metrics collection (default: {TIME_WINDOW})')
    parser.add_argument('--no-health', action='store_false', dest='monitor_health',
                       default=MONITOR_NODE_HEALTH, 
                       help='Disable node health monitoring (enabled by default)')
    parser.add_argument('--no-performance', action='store_false', dest='monitor_performance',
                       default=MONITOR_FUNCTION_PERFORMANCE, 
                       help='Disable function performance monitoring (enabled by default)')
    parser.add_argument('--no-clean', action='store_false', dest='clean_cli',
                       default=CLEAN_CLI, 
                       help='Disable CLI cleaning between updates (enabled by default)')
    
    # Output settings
    parser.add_argument('--no-csv', action='store_false', dest='write_csv',
                       default=OUTPUT_WRITE_TO_CSV, 
                       help='Disable CSV output (enabled by default)')
    parser.add_argument('--experiment-name', type=str, default=OUTPUT_EXPERIMENT_NAME, 
                       help=f'Experiment name for output files (default: {OUTPUT_EXPERIMENT_NAME})')
    parser.add_argument('--no-columns', action='store_false', dest='output_columns',
                       default=OUTPUT_COLUMNS, 
                       help='Disable columns in CSV output (enabled by default)')
    
    # Model settings
    parser.add_argument('--model-path', type=validate_model_path, default=DEFAULT_MODEL_PATH,
                       help=f'Path to the ML model file (default: {DEFAULT_MODEL_PATH})')
    
    args = parser.parse_args()

    # Create configuration dictionary
    config = {
        'proxy_host': args.proxy_host,
        'proxy_port': args.proxy_port,
        'proxy_db': args.proxy_db,
        'interval': args.interval,
        'time_window': args.time_window,
        'monitor_health': args.monitor_health,
        'monitor_performance': args.monitor_performance,
        'clean_cli': args.clean_cli,
        'write_csv': args.write_csv,
        'experiment_name': args.experiment_name,
        'output_columns': args.output_columns,
        'model_path': args.model_path
    }

    # Log configuration
    logger.info("Starting with configuration:")
    for key, value in config.items():
        logger.info(f"  {key}: {value}")

    return config
