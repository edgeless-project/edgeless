# src/__init__.py
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
    - EDGELESS_AD_PROXY_HOST: PROXY server host (default: 127.0.0.1)
    - EDGELESS_AD_PROXY_PORT: PROXY server port (default: 6379)
    - EDGELESS_AD_PROXY_DB:   PROXY server database number (default: 0)

    - EDGELESS_AD_INFERENCE_MODEL_FILE:      Path to pre-trained ML model pickle file
    - EDGELESS_AD_INFERENCE_PERIOD:          Inference interval in seconds (default: 5)
    - EDGELESS_AD_INFERENCE_WINDOW:          Time window in seconds to be evaulated (default: 1800)
    - EDGELESS_AD_INFERENCE_STOP_WHEN_FOUND: Stop the application when the first anomaly is found (default: false)
    - EDGELESS_AD_INFERENCE_DISABLE:         Inference is disabled, but DataFrames are still being generated. Only used for debugging purposes (default: false)

    - EDGELESS_AD_LOG_LEVEL_DEBUG:   Logging level for both stdout and logfiles. False = INFO, True = DEBUG (default: false)
    - EDGELESS_AD_LOG_WRITE_TO_DISK: Write logs to disk. Disables the next 3 variables if false (default: false)
    - EDGELESS_AD_LOG_DIR:           Directory to write log files (default: <edgeless_path>/anomaly_detection/outputs/logs/)
    - EDGELESS_AD_LOG_PREFIX:        Prefix for log filenames. E.g. '<EDGELESS_AD_LOG_PREFIX>-20251003_1023.log' (default: '')
    - EDGELESS_AD_LOG_FULLNAME:      Fullname for log filename. Overwrites and ignores 'EDGELESS_AD_LOG_PREFIX'. E.g. '<EDGELESS_AD_LOG_FULLNAME>.log' (default: '')

    - EDGELESS_AD_DF_WRITE_TO_CSV:     Write latest DataFrame to a CSV file (default: false)
    - EDGELESS_AD_DF_WRITE_TO_PARQUET: Write latest DataFrame to a Parquet file (default: false)
    - EDGELESS_AD_DF_INCLUDE_COLUMNS:  Include column headers in output CSV files (default: true)
    - EDGELESS_AD_DF_DIR:              Directory to write DataFrame files (default: <edgeless_path>/anomaly_detection/outputs/dataframes/)
    - EDGELESS_AD_DF_PREFIX:           Prefix for DataFrame filenames. E.g. '<EDGELESS_AD_DF_PREFIX>_20251003_1023_node_health_df.csv' (default: '')
    - EDGELESS_AD_DF_FULLNAME:         Fullname for DataFrame filename. Overwrites and ignores 'EDGELESS_AD_DF_PREFIX'. E.g. '<EDGELESS_AD_DF_FULLNAME>_node_health_df.csv' (default: '')
"""

__version__ = "0.1.0"
__author__ = "alvaro.curtomerino@telefonica.com"
__description__ = "Real-time anomaly detection system for EDGELESS systems"
