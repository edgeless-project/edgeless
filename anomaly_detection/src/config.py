#!/usr/bin/env python3

import os
from pathlib import Path

class Config:    
    def __init__(self):
        project_root = Path(__file__).resolve().parents[2]

        # Redis connection settings
        self.EDGELESS_AD_PROXY_HOST = os.getenv('EDGELESS_AD_PROXY_HOST', '127.0.0.1')
        self.EDGELESS_AD_PROXY_PORT = int(os.getenv('EDGELESS_AD_PROXY_PORT', 6379))
        self.EDGELESS_AD_PROXY_DB   = int(os.getenv('EDGELESS_AD_PROXY_DB', 0))
        
        # Model and inference settings
        self.EDGELESS_AD_INFERENCE_MODEL_FILE = os.getenv('EDGELESS_AD_INFERENCE_MODEL_FILE', 'outputs/models/random_model.pkl')
        self.EDGELESS_AD_INFERENCE_PERIOD = float(os.getenv('EDGELESS_AD_INFERENCE_PERIOD', '5'))
        self.EDGELESS_AD_INFERENCE_WINDOW = float(os.getenv('EDGELESS_AD_INFERENCE_WINDOW', '1800'))
        self.EDGELESS_AD_INFERENCE_STOP_WHEN_FOUND = os.getenv('EDGELESS_AD_INFERENCE_STOP_WHEN_FOUND', 'false').lower() == 'true'
        self.EDGELESS_AD_INFERENCE_DISABLE = os.getenv('EDGELESS_AD_INFERENCE_DISABLE', 'false').lower() == 'true'
        
        # File-based mapping settings
        self.EDGELESS_AD_USE_FILE_MAPPING = os.getenv('EDGELESS_AD_USE_FILE_MAPPING', 'false').lower() == 'true'
        self.EDGELESS_AD_MAPPING_FILE_PATH = Path(os.getenv('EDGELESS_AD_MAPPING_FILE_PATH', project_root / "mapping_to_instance_id.csv"))
        
        # Display and debug settings
        self.EDGELESS_AD_LOG_LEVEL_DEBUG   = os.getenv('EDGELESS_AD_LOG_LEVEL_DEBUG', 'false').lower() == 'true'
        self.EDGELESS_AD_LOG_WRITE_TO_DISK = os.getenv('EDGELESS_AD_LOG_WRITE_TO_DISK', 'false').lower() == 'true'
        self.EDGELESS_AD_LOG_DIR = Path( os.getenv("EDGELESS_AD_LOG_DIR", project_root / "outputs" / "logs") )
        self.EDGELESS_AD_LOG_PREFIX   = os.getenv('EDGELESS_AD_LOG_PREFIX', '')
        self.EDGELESS_AD_LOG_FULLNAME = os.getenv('EDGELESS_AD_LOG_FULLNAME', '')
    
        # DataFrame static analysis settings
        self.EDGELESS_AD_DF_WRITE_TO_CSV     = os.getenv('EDGELESS_AD_DF_WRITE_TO_CSV', 'false').lower() == 'true'
        self.EDGELESS_AD_DF_WRITE_TO_PARQUET = os.getenv('EDGELESS_AD_DF_WRITE_TO_PARQUET', 'false').lower() == 'true'
        self.EDGELESS_AD_DF_INCLUDE_COLUMNS  = os.getenv('EDGELESS_AD_DF_INCLUDE_COLUMNS', 'true').lower() == 'true'
        self.EDGELESS_AD_DF_DIR = Path( os.getenv("EDGELESS_AD_DF_DIR", project_root / "outputs" / "dataframes") )
        self.EDGELESS_AD_DF_PREFIX   = os.getenv('EDGELESS_AD_DF_PREFIX', '')
        self.EDGELESS_AD_DF_FULLNAME = os.getenv('EDGELESS_AD_DF_FULLNAME', '')


        # Create output directories if needed
        if self.EDGELESS_AD_LOG_WRITE_TO_DISK:
            self.EDGELESS_AD_LOG_DIR.mkdir(parents=True, exist_ok=True)
        
        if self.EDGELESS_AD_DF_WRITE_TO_CSV or self.EDGELESS_AD_DF_WRITE_TO_PARQUET:
            self.EDGELESS_AD_DF_DIR.mkdir(parents=True, exist_ok=True)


    def __str__(self):
        vars_dict = vars(self)
        config_lines = [f"  {k}: {v}" for k, v in vars_dict.items()]
        return "\n".join(config_lines)
