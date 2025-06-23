#!/usr/bin/env python3

import logging
from typing import Dict, List, Tuple, Optional
import pandas as pd
from datetime import datetime
import numpy as np
import json

from config import Config


NODE_HEALTH_COLUMNS = {
    'mem_free', 'mem_used', 'mem_available', 'proc_cpu_usage', 
    'proc_memory', 'proc_vmemory', 'load_avg_1', 'load_avg_5', 
    'load_avg_15', 'tot_rx_bytes', 'tot_rx_pkts', 'tot_rx_errs', 
    'tot_tx_bytes', 'tot_tx_pkts', 'tot_tx_errs', 'disk_free_space', 
    'disk_tot_reads', 'disk_tot_writes', 'gpu_load_perc', 
    'gpu_temp_cels', 'active_power'
}


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
        try:
            return key.split(":")[-1]
        except IndexError:
            return None
        

    def node_health_data_to_dataframe(self, data_dict: Dict[str, List[Tuple[str, float]]]) -> pd.DataFrame:
        """
        Convert node health data from Redis to pandas DataFrame.
        
        Args:
            data_dict (Dict): Dictionary with Redis key names and their health data
            
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
    

    def node_health_data_to_dataframe(self, data_dict: Dict[str, List[Tuple[str, float]]]) -> pd.DataFrame:
        """
        Convert node health data from Redis to pandas DataFrame.
        
        Args:
            data_dict (Dict): Dictionary with Redis key names and their health data
            
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
                    # Parsear los datos de salud del JSON
                    health_data = self._parse_health_data(member)
                    
                    if not health_data:
                        self.logger.warning(f"Skipping empty health data for node {node_uuid}")
                        continue
                    
                    # Validar y limpiar los datos
                    validated_data = self._validate_health_data(health_data)
                    
                    # Crear el registro base
                    record = {
                        'timestamp': score,
                        'node_uuid': node_uuid,
                        **validated_data  # Expandir todas las métricas de salud
                    }
                    
                    all_records.append(record)
            
            if not all_records:
                self.logger.warning("No valid records found after processing")
                return pd.DataFrame()
            
            # Crear DataFrame
            df = pd.DataFrame(all_records)
            
            # Convertir timestamp a datetime si es necesario
            if 'timestamp' in df.columns:
                try:
                    df['timestamp'] = pd.to_datetime(df['timestamp'], unit='s', errors='coerce')
                except Exception as e:
                    self.logger.warning(f"Could not convert timestamp to datetime: {e}")
            
            # Ordenar por timestamp y node_uuid para mejor organización
            df = df.sort_values(['node_uuid', 'timestamp'])
            
            # Resetear índice
            df = df.reset_index(drop=True)
            
            self.logger.info(f"Successfully processed {len(df)} health records for {df['node_uuid'].nunique()} nodes")
            
            return df

        except Exception as e:
            self.logger.error(f"Error converting node health data to DataFrame: {str(e)}", exc_info=True)
            return pd.DataFrame()
        


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
    

    # Currently unused
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
    

    # NOTE: The file is constantly being overwritten. It allows to analyze the dataframes where the model finded or not an anomaly
    def save_to_csv(self, health_df: pd.DataFrame, performance_df: pd.DataFrame):
        """
        Save DataFrames to CSV files if enabled.
        
        Args:
            health_df (pd.DataFrame): Health metrics DataFrame
            performance_df (pd.DataFrame): Performance metrics DataFrame
            timestamp (str): Timestamp for file naming
        """        
        try:
            base_path = f"outputs/{self.config.OUTPUT_EXPERIMENT_NAME}"
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            
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