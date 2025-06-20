#!/usr/bin/env python3

import logging
from typing import Dict, List, Tuple, Optional
import pandas as pd
from datetime import datetime

# import numpy as np

from config import Config




class DataProcessor:    
    def __init__(self, config: Config):
        self.config = config
        self.logger = logging.getLogger(__name__)
    

    def redis_data_to_dataframe(self, data_dict: Dict[str, List[Tuple[str, float]]], prefix: str) -> pd.DataFrame:
        """
        Convert Redis data dictionary to pandas DataFrame.
        
        Args:
            data_dict (Dict): Dictionary with Redis key names and their data
            prefix (str): Prefix for column naming
            
        Returns:
            pd.DataFrame: Processed DataFrame
        """
        if not data_dict:
            return pd.DataFrame()
        
        try:
            all_records = []
            
            for key, members in data_dict.items():
                for member, score in members:
                    all_records.append({
                        'key': key,
                        'member': member,
                        'timestamp': score,
                        'datetime': datetime.fromtimestamp(score),
                        'prefix': prefix
                    })
            
            if not all_records:
                return pd.DataFrame()
            
            df = pd.DataFrame(all_records)
            df = df.sort_values('timestamp')
            
            return df
            
        except Exception as e:
            self.logger.error(f"Error converting data from the PROXY server into a DataFrame: {str(e)}")
            return pd.DataFrame()
    


    def prepare_features(self, health_df: pd.DataFrame, performance_df: pd.DataFrame) -> Optional[np.ndarray]:
        """
        Prepare feature matrix for ML model from health and performance DataFrames.
        
        Args:
            health_df (pd.DataFrame): Health metrics DataFrame
            performance_df (pd.DataFrame): Performance metrics DataFrame
            
        Returns:
            Optional[np.ndarray]: Feature matrix or None if no data
        """
        try:
            features = []
            
            # Basic statistics from health data
            if not health_df.empty:
                features.extend([
                    len(health_df),  # Number of health records
                    health_df['key'].nunique(),  # Number of unique health keys
                    health_df['timestamp'].max() - health_df['timestamp'].min() if len(health_df) > 1 else 0,  # Time span
                ])
            else:
                features.extend([0, 0, 0])
            
            # Basic statistics from performance data
            if not performance_df.empty:
                features.extend([
                    len(performance_df),  # Number of performance records
                    performance_df['key'].nunique(),  # Number of unique performance keys
                    performance_df['timestamp'].max() - performance_df['timestamp'].min() if len(performance_df) > 1 else 0,  # Time span
                ])
            else:
                features.extend([0, 0, 0])
            
            # Add more sophisticated features based on your specific needs
            # For example: rate of change, patterns, etc.
            
            if not features or all(f == 0 for f in features):
                return None
            
            return np.array(features).reshape(1, -1)
            
        except Exception as e:
            self.logger.error(f"Error preparing features: {str(e)}")
            return None
    
    def save_to_csv(self, health_df: pd.DataFrame, performance_df: pd.DataFrame, timestamp: str):
        """
        Save DataFrames to CSV files if enabled.
        
        Args:
            health_df (pd.DataFrame): Health metrics DataFrame
            performance_df (pd.DataFrame): Performance metrics DataFrame
            timestamp (str): Timestamp for file naming
        """
        if not self.config.OUTPUT_WRITE_TO_CSV:
            return
        
        try:
            base_path = f"outputs/{self.config.OUTPUT_EXPERIMENT_NAME}"
            
            if not health_df.empty:
                health_file = f"{base_path}/health_data_{timestamp}.csv"
                health_df.to_csv(health_file, index=False, header=self.config.OUTPUT_COLUMNS)
                self.logger.debug(f"Saved health data to {health_file}")
            
            if not performance_df.empty:
                performance_file = f"{base_path}/performance_data_{timestamp}.csv"
                performance_df.to_csv(performance_file, index=False, header=self.config.OUTPUT_COLUMNS)
                self.logger.debug(f"Saved performance data to {performance_file}")
                
        except Exception as e:
            self.logger.error(f"Error saving CSV files: {str(e)}")