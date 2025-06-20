#!/usr/bin/env python3

import logging
import redis
import time
from typing import Dict, List, Tuple

from config import Config


class ProxyMonitor:    
    def __init__(self, config: Config):
        self.config = config
        self.logger = logging.getLogger(__name__)
        self.proxy_client = None
        self.health_data = {}
        self.performance_data = {}
        

    def connect(self) -> bool:
        """
        Establish connection to Redis proxy server.
        
        Returns:
            bool: True if connection successful, False otherwise
        """
        try:
            self.proxy_client = redis.Redis(
                host=self.config.PROXY_HOST,
                port=self.config.PROXY_PORT,
                db=self.config.PROXY_DB,
                decode_responses=True,
                socket_timeout=5,
                socket_connect_timeout=5
            )
            
            # Test connection
            self.proxy_client.ping()
            self.logger.info(f"Connected to PROXY server at redis://{self.config.PROXY_HOST}:{self.config.PROXY_PORT}")
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to connect to PROXY server: {str(e)}")
            return False


    def get_sorted_set_data(self, pattern: str, time_window: float) -> Dict[str, List[Tuple[str, float]]]:
        """
        Retrieve data from Redis sorted sets matching the pattern within time window.
        
        Args:
            pattern (str): Pattern to match Redis keys
            time_window (float): Time window in seconds
            
        Returns:
            Dict[str, List[Tuple[str, float]]]: Dictionary with key names and their data
        """
        # TODO: Code will most likely break here, due to format issues
        try:
            current_time = time.time()
            cutoff_time = current_time - time_window
            
            # Find all keys matching pattern
            keys = self.proxy_client.keys(f"{pattern}*")
            
            data = {}
            for key in keys:
                try:
                    # Get data from sorted set within time window
                    # ZRANGEBYSCORE returns members with scores between cutoff_time and current_time
                    members = self.proxy_client.zrangebyscore(
                        key, 
                        cutoff_time, 
                        current_time, 
                        withscores=True
                    )
                    
                    if members:
                        data[key] = members
                        
                except Exception as e:
                    self.logger.warning(f"Error retrieving data from key {key}: {str(e)}")
                    
            return data
            
        except Exception as e:
            self.logger.error(f"Error getting sorted set data for pattern {pattern}: {str(e)}")
            return {}
    

    def monitor_metrics(self):
        self.logger.info("=== Starting PROXY server monitoring... ===")
        
        try:
            # Get health and performance data
            self.health_data = self.get_sorted_set_data("health:status:", self.config.AD_TIME_WINDOW)
            self.performance_data = self.get_sorted_set_data("performance:", self.config.AD_TIME_WINDOW)
            
        except Exception as e:
            self.logger.error(f"Error retrieving metrics from the PROXY server: {str(e)}")
    

    def get_current_data(self) -> Tuple[Dict, Dict]:
        """
        Get current snapshot of monitored data.
        
        Returns:
            Tuple[Dict, Dict]: Current health and performance data
        """
        return self.health_data.copy(), self.performance_data.copy()