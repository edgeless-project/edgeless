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

        self.node_capabilities_last_update = None
        self.instance_last_update = None
        self.dependency_last_update = None

        self.node_capabilities_data = {}
        self.instance_data = {}
        self.dependency_data = {}
        self.node_health_data = {}
        self.performance_function_execution_time_data = {}
        self.performance_function_transfer_time_data = {}
        

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
        

    def get_domain_id(self):
        """
        Retrieve the domain ID from the PROXY server.
        Returns:
            str: Domain ID if available, None otherwise
        """
        try:
            return self.proxy_client.get("domain_info:domain_id")
        except Exception as e:
            self.logger.error(f"Error retrieving domain_id from PROXY server: {str(e)}")
        return None


    def update_static_data(self, data: str, pattern: str, last_update_pattern: str) -> None:
        """
        Updates the cache for static data like node capabilities, instance info, and dependencies
        by checking their last update timestamp.

        Args:
            data (str): Type of data to update (e.g., "node_capabilities", "instance", "dependency")
            pattern (str): Key pattern to match in the PROXY server (e.g., "node:capabilities:*", "instance:*", "dependency:*")
            last_update_pattern (str): Key pattern for the last update timestamp in the PROXY server
        """
        try:
            current_update = self.proxy_client.get(f"{last_update_pattern}")
            last_update = getattr(self, f"{data}_last_update", None)

            if current_update == last_update:
                # Nothing changed. Proceed with cached data
                return
            
            # Data has changed, fetch all instance keys
            keys = self.proxy_client.keys(f"{pattern}")
            data_dict = {}
            
            for key in keys:
                try:
                    if key.endswith(":last_update"):
                        continue
                    value = self.proxy_client.get(key)
                    if value is not None:
                        data_dict[key] = value
                        
                except Exception as e:
                    self.logger.warning(f"Error retrieving instance data from key {key}: {str(e)}")
            
            # Update cache
            setattr(self, f"{data}_data", data_dict)
            setattr(self, f"{data}_last_update", current_update)
            return
            
        except Exception as e:
            self.logger.error(f"Error updating static data for {data}: {str(e)}")
            return
        

    def update_sorted_set_data(self, data: str, pattern: str) -> None:
        """
        Updates the cache for sorted set data like node health and performance metrics
        by retrieving data within a specified time window.

        Args:
            data (str): Type of data to update (e.g., "node_health", "performance")
            pattern (str): Key pattern to match in the PROXY server (e.g., "node:health:*", "performance:*:function_execution_time")
        """
        try:
            current_time = time.time()
            cutoff_time = current_time - self.config.AD_TIME_WINDOW
            
            # Find all keys matching pattern
            keys = self.proxy_client.keys(f"{pattern}")
            
            data_dict = {}
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
                        data_dict[key] = members
                        
                except Exception as e:
                    self.logger.warning(f"Error retrieving data from key {key}: {str(e)}")

            # Update cache
            setattr(self, f"{data}_data", data_dict)       
            return
            
        except Exception as e:
            self.logger.error(f"Error updating sorted set data for {data}: {str(e)}")
            return
    
    

    def update_data(self) -> Tuple[Dict, Dict]:
        """
        Updates the class attributes according to the latest data from the PROXY server.
        """
        try:
        
            self.update_static_data("node_capabilities", "node:capabilities:*", "node:capabilities:last_update")
            self.update_static_data("instance", "instance:*", "instance:last_update")
            self.update_static_data("dependency", "dependency:*", "dependency:last_update")

            self.update_sorted_set_data("node_health", "node:health:*")
            self.update_sorted_set_data("performance_function_execution_time", "performance:*:function_execution_time")
            self.update_sorted_set_data("performance_function_transfer_time", "performance:*:function_transfer_time")
            
        except Exception as e:
            self.logger.error(f"Error retrieving metrics from the PROXY server: {str(e)}")

        return
    
    def get_data(self, data: str) -> Dict[str, List[Tuple[str, float]]]:
        """
        Retrieve data from the cache based on the type.
        
        Args:
            data (str): Type of data to retrieve (e.g., "node_health", "performance")
        
        Returns:
            Dict[str, List[Tuple[str, float]]]: Cached data for the specified type
        """
        if hasattr(self, f"{data}_data"):
            return getattr(self, f"{data}_data", {})
        else:
            self.logger.error(f"Unknown data type requested: {data}")
            return {}