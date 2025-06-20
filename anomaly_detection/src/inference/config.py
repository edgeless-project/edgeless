#!/usr/bin/env python3

import os

class Config:    
    def __init__(self):
        # Redis connection settings
        self.PROXY_HOST = os.getenv('PROXY_HOST', '127.0.0.1')
        self.PROXY_PORT = int(os.getenv('PROXY_PORT', 6379))
        self.PROXY_DB = int(os.getenv('PROXY_DB', 0))
        
        # Model and inference settings
        self.AD_MODEL_FILE = os.getenv('AD_MODEL_FILE', 'outputs/models/random_model.pkl')
        self.AD_INFERENCE_PERIOD = float(os.getenv('AD_INFERENCE_PERIOD', '5'))
        self.AD_TIME_WINDOW = float(os.getenv('AD_TIME_WINDOW', '1800'))  # 30 minutes in seconds
        
        # Display and debug settings
        self.CLEAN_CLI = os.getenv('CLEAN_CLI', 'true').lower() == 'true'
        self.DEBUG = os.getenv('DEBUG', 'false').lower() == 'true'
        
        # Output settings
        self.OUTPUT_WRITE_TO_CSV = os.getenv('OUTPUT_WRITE_TO_CSV', 'false').lower() == 'true'
        self.OUTPUT_EXPERIMENT_NAME = os.getenv('OUTPUT_EXPERIMENT_NAME', 'edgeless_experiment')
        self.OUTPUT_COLUMNS = os.getenv('OUTPUT_COLUMNS', 'true').lower() == 'true'
        
        # Create output directory if needed
        if self.OUTPUT_WRITE_TO_CSV:
            os.makedirs(f"outputs/{self.OUTPUT_EXPERIMENT_NAME}", exist_ok=True)

    def __str__(self):
        vars_dict = vars(self)
        config_lines = [f"  {k}: {v}" for k, v in vars_dict.items()]
        return "\n".join(config_lines)


# PROXY_HOST = os.getenv('PROXY_HOST', '127.0.0.1')
# PROXY_PORT = int(os.getenv('PROXY_PORT', 6379))
# PROXY_DB = int(os.getenv('PROXY_DB', 0))
# AD_MODEL_FILE = os.getenv('AD_MODEL_FILE', 'outputs/models/random_model.pkl')
# AD_INFERENCE_PERIOD = float(os.getenv('AD_INFERENCE_PERIOD', 5))
# AD_TIME_WINDOW = int(os.getenv('AD_TIME_WINDOW', 1800))  # 30 mins default
# CLEAN_CLI = os.getenv('CLEAN_CLI', 'true').lower() == 'true'
# DEBUG = os.getenv('DEBUG', 'false').lower() == 'true'
# OUTPUT_WRITE_TO_CSV = os.getenv('OUTPUT_WRITE_TO_CSV', 'false').lower() == 'true'
# OUTPUT_EXPERIMENT_NAME = os.getenv('OUTPUT_EXPERIMENT_NAME', 'experiment')
# OUTPUT_COLUMNS = os.getenv('OUTPUT_COLUMNS', 'false').lower() == 'true'