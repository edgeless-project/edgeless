#!/usr/bin/env python3

import logging
from typing import Dict, List, Tuple, Optional
import pandas as pd
from datetime import datetime
import numpy as np
import re
import json

from config import Config


# NODE_HEALTH_COLUMNS = {
#     'mem_free', 'mem_used', 'mem_available', 'proc_cpu_usage', 
#     'proc_memory', 'proc_vmemory', 'load_avg_1', 'load_avg_5', 
#     'load_avg_15', 'tot_rx_bytes', 'tot_rx_pkts', 'tot_rx_errs', 
#     'tot_tx_bytes', 'tot_tx_pkts', 'tot_tx_errs', 'disk_free_space', 
#     'disk_tot_reads', 'disk_tot_writes', 'gpu_load_perc', 
#     'gpu_temp_cels', 'active_power'
# }

# FUNCTION_PERFORMANCE_COLUMNS = [
#     "timestamp", "type", "duration", "physical_uuid",
#     "node_uuid", "logical_uuid", "workflow_uuid", "class_id"
# ]


class DataProcessor:    
    def __init__(self, config: Config):
        self.config = config
        self.logger = logging.getLogger(__name__)
    

    def extract_uuid(self, key: str) -> Optional[str]:
        """
        Extract UUID from Redis key.
        Args:
            key (str): Redis key
        Returns:
            str: Extracted UUID or None if not found
        """
        # UUID v4 standard (with  dashes)
        uuid_pattern = re.compile(r"[a-f0-9]{8}-[a-f0-9]{4}-[1-5][a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}", re.IGNORECASE)

        match = uuid_pattern.search(key)
        return match.group(0) if match else None
        

    def function_performance_parse(self, entry: str) -> Optional[float]:
        try:
            _, value = map(float, entry.split(":"))
            return value
        except ValueError:
            return None


    def node_health_data_to_dataframe(self, data_dict: Dict[str, List[Tuple[str, float]]]) -> pd.DataFrame:
        """
        Convert node health data into a pandas DataFrame.
        
        Args:
            data_dict (Dict): Dictionary with key names and the node health data
            
        Returns:
            pd.DataFrame: Processed DataFrame
        """
        if not data_dict:
            return pd.DataFrame()
        
        try:
            all_records = []
            
            for key, members in data_dict.items():

                node_uuid = self.extract_uuid(key)
                if not node_uuid:
                    self.logger.warning(f"Could not extract UUID from key: {key}")
                    continue

                for member, score in members:
                    try:
                        # Remove all double quotes to avoid JSON parsing issues
                        cleaned_member = json.loads(member.replace('""', '"'))
                    except (json.JSONDecodeError, AttributeError) as e:
                        self.logger.warning(f"Failed to parse health data: {member[:100]}... Error: {e}")
                        return {}
        
                    if not cleaned_member:
                        self.logger.warning(f"Skipping empty health data for node {node_uuid}")
                        continue

                    # Crear el registro base
                    all_records.append({
                        'timestamp': score,
                        'node_uuid': node_uuid,
                        **cleaned_member  # Expandir todas las métricas de salud
                    })
            
            if not all_records:
                return pd.DataFrame()
            
            df = pd.DataFrame(all_records)
            df = df.sort_values('timestamp')
            
            return df

        except Exception as e:
            self.logger.error(f"Error converting node health data to a DataFrame: {str(e)}")
            return pd.DataFrame()
        

    def performance_data_to_dataframe(self, function_execution_time_dict: Dict[str, List[Tuple[str, float]]], function_transfer_time_dict: Dict[str, List[Tuple[str, float]]]) -> pd.DataFrame:
        """
        Convert performance data into a pandas DataFrame.
        
        Args:
            function_execution_time_dict (Dict): Dictionary with function execution time data
            function_transfer_time_dict (Dict): Dictionary with function transfer time data
            
        Returns:
            pd.DataFrame: Processed DataFrame
        """        
        try:
            all_records = []

            for key, members in function_execution_time_dict.items():
            
                physical_uuid = self.extract_uuid(key)

                for member, score in members:

                    time = self.function_performance_parse(member)
                    all_records.append({
                        'timestamp': score,
                        'performance_measurement_type': 'function_execution_time',
                        'value': time,
                        'physical_uuid': physical_uuid,
                    })

            for key, members in function_transfer_time_dict.items():
            
                physical_uuid = self.extract_uuid(key)

                for member, score in members:

                    time = self.function_performance_parse(member)
                    all_records.append({
                        'timestamp': score,
                        'performance_measurement_type': 'function_transfer_time',
                        'value': time,
                        'physical_uuid': physical_uuid,
                    })
            
            if not all_records:
                return pd.DataFrame()
            
            df = pd.DataFrame(all_records)
            df = df.sort_values('timestamp')
            
            return df

        except Exception as e:
            self.logger.error(f"Error converting performance data to a DataFrame: {str(e)}")
            return pd.DataFrame()
    

    # # Currently unused
    # def prepare_features(self, health_df: pd.DataFrame, performance_df: pd.DataFrame) -> Optional[np.ndarray]:
    #     """
    #     Prepare feature matrix for ML model from health and performance DataFrames.
        
    #     Args:
    #         health_df (pd.DataFrame): Health metrics DataFrame
    #         performance_df (pd.DataFrame): Performance metrics DataFrame
            
    #     Returns:
    #         Optional[np.ndarray]: Feature matrix or None if no data
    #     """
    #     try:
    #         features = []
            
    #         # Basic statistics from health data
    #         if not health_df.empty:
    #             features.extend([
    #                 len(health_df),  # Number of health records
    #                 health_df['key'].nunique(),  # Number of unique health keys
    #                 health_df['timestamp'].max() - health_df['timestamp'].min() if len(health_df) > 1 else 0,  # Time span
    #             ])
    #         else:
    #             features.extend([0, 0, 0])
            
    #         # Basic statistics from performance data
    #         if not performance_df.empty:
    #             features.extend([
    #                 len(performance_df),  # Number of performance records
    #                 performance_df['key'].nunique(),  # Number of unique performance keys
    #                 performance_df['timestamp'].max() - performance_df['timestamp'].min() if len(performance_df) > 1 else 0,  # Time span
    #             ])
    #         else:
    #             features.extend([0, 0, 0])
            
    #         # Add more sophisticated features based on your specific needs
    #         # For example: rate of change, patterns, etc.
            
    #         if not features or all(f == 0 for f in features):
    #             return None
            
    #         return np.array(features).reshape(1, -1)
            
    #     except Exception as e:
    #         self.logger.error(f"Error preparing features: {str(e)}")
    #         return None
    

    # NOTE: The file is constantly being overwritten. It allows to analyze the dataframe where the model finded or not an anomaly
    def save_to_csv(self, df: pd.DataFrame, file_name: str):
        """
        Save DataFrames to CSV files if enabled.
        
        Args:
            df (pd.DataFrame): DataFrame to save
            file_name (str): Name of the DataFrame (e.g., "node_health_data", "performance_data")
        """        
        try:
            base_path = f"outputs/{self.config.OUTPUT_EXPERIMENT_NAME}"
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            
            if not df.empty:
                csv_file = f"{base_path}/{file_name}_{timestamp}.csv"
                df.to_csv(csv_file, index=False, header=self.config.OUTPUT_COLUMNS)
                self.logger.debug(f"Saved health data to {csv_file}")
                
        except Exception as e:
            self.logger.error(f"Error saving CSV files: {str(e)}")


    def save_to_parquet(self, df: pd.DataFrame, file_name: str):
        """
        Save DataFrames to CSV files if enabled.
        
        Args:
            df (pd.DataFrame): DataFrame to save
            file_name (str): Name of the DataFrame (e.g., "node_health_data", "performance_data")
        """        
        try:
            base_path = f"outputs/{self.config.OUTPUT_EXPERIMENT_NAME}"
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            
            if not df.empty:
                parquet_file = f"{base_path}/{file_name}_{timestamp}.parquet"
                df.to_parquet(parquet_file, index=False)
                self.logger.debug(f"Saved health data to {parquet_file}")
                
        except Exception as e:
            self.logger.error(f"Error saving Parquet files: {str(e)}")