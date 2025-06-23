"""
EDGELESS Anomaly Detection System

A modular system for real-time anomaly detection using metrics monitoring from the PROXY server
and machine learning inference.

Modules:
    - config: Configuration management
    - proxy_monitor: PROXY server connection and data monitoring (currently supports Redis or similar)
    - data_processor: Data transformation and feature engineering
    - anomaly_detector: ML model interface and prediction
    - main: Main orchestration system

Usage:
    python main.py

Environment Variables:
    - PROXY_HOST: PROXY server host (default: 127.0.0.1)
    - PROXY_PORT: PROXY server port (default: 6379)
    - PROXY_DB: PROXY server database number (default: 0)
    - AD_MODEL_FILE: Path to pre-trained ML model pickle file
    - AD_INFERENCE_PERIOD: Inference interval in seconds (default: 5)
    - AD_TIME_WINDOW: Data time window in seconds to be evaulated (default: 1800)
    - CLEAN_CLI: Clear CLI output between updates (default: true)
    - DEBUG: Enable debug output (default: false)
    - OUTPUT_WRITE_TO_CSV: Write output to CSV files (default: false)
    - OUTPUT_WRITE_TO_PARQUET: Write output to Parquet files (default: false)
    - OUTPUT_EXPERIMENT_NAME: Name for output files (default: experiment)
    - OUTPUT_COLUMNS: Include column headers in CSV output (default: false)
"""

__version__ = "0.1.0"
__author__ = "alvaro.curtomerino@telefonica.com"
__description__ = "Real-time anomaly detection system for EDGELESS systems"

# Import main classes for easy access
from .config import Config
from .proxy_monitor import ProxyMonitor
from .data_processor import DataProcessor
from .anomaly_detector import AnomalyDetector
from .main import EDGELESSAnomalyDetectionInferer

__all__ = [
    'Config',
    'ProxyMonitor', 
    'DataProcessor',
    'AnomalyDetector',
    'EDGELESSAnomalyDetectionInferer'
]