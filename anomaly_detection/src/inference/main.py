#!/usr/bin/env python3

import logging
from datetime import datetime
import signal
import os
import time
import pandas as pd
from typing import Dict, Any
import sys
import random  # Delete this import in the future

<<<<<<< HEAD
from config import Config
from proxy_monitor import ProxyMonitor
from data_processor import DataProcessor
from anomaly_detector import AnomalyDetector
from models.random_binary_model import RandomBinaryModel
=======
### Definition of environmen definition variables
LOOP_PERIOD = 2
REDIS_HOST = "10.95.82.180"
REDIS_PORT = 6379
REDIS_DB = 0
TIME_WINDOW = 60            # metrics collected up to TIME_WINDOW seconds old
MONITOR_NODE_HEALTH = True
MONITOR_FUNCTION_PERFORMANCE = True
CLEAN_CLI = True

OUTPUT_WRITE_TO_CSV = True
OUTPUT_EXPERIMENT_NAME = "mapreduce_flat"
OUTPUT_COLUMNS = True

### Columns for the pandas dataframes
node_health_columns = [
    "timestamp", "node_uuid", "mem_free", "mem_used", "mem_available",
    "proc_cpu_usage", "proc_memory", "proc_vmemory",
    "load_avg_1", "load_avg_5", "load_avg_15",
    "tot_rx_bytes", "tot_rx_pkts", "tot_rx_errs",
    "tot_tx_bytes", "tot_tx_pkts", "tot_tx_errs",
    "disk_free_space", "disk_tot_reads", "disk_tot_writes",
    "gpu_load_perc", "gpu_temp_cels"
]
function_performance_columns = [
    "timestamp", "type", "duration", "physical_uuid",
    "node_uuid", "logical_uuid", "workflow_uuid", "class_id"
]

### Aux function to extract UUIDs from a redis key
def extract_uuid(key):
    try:
        return key.split(":")[-1]
    except IndexError:
        return None

### Aux function to remove the timestamp from function performance metrics, as this information is already in the score
def function_performance_parse(entry):
    try:
        _, value = map(float, entry.split(":"))
        return value
    except ValueError:
        return None, None

def get_functions_info(redis_client):
    """
    Retrieves function information from Redis and organizes it into a dictionary.
    """
    functions_info = {}

    try:
        if not (function_keys := redis_client.keys("instance:*")):
            print("ERROR: No instance:* keys found. Exiting...")
            return functions_info  # Empty dictionary

        for key in function_keys:
            logical_uuid = key.split(":")[-1]

            # Skip last_update entry
            if "last_update" in logical_uuid:
                continue
              
            json_data = redis_client.get(key)

            if not json_data:
                print(f"ERROR: No data found for key {key}")
                continue

            try:
                decoded_json = json.loads(json_data)
            except json.JSONDecodeError:
                print(f"ERROR: Failed to decode JSON for {key}")
                continue

            # Skip 'resource' instances
            if "Function" not in decoded_json:
                print(f"Skipping key {key}, does not contain 'Function'")
                continue

            original_data = decoded_json["Function"]
            if not original_data or len(original_data) < 2:
                print(f"ERROR: Unexpected structure in function data for key {key}")
                continue

            metadata = original_data[0]
            instance_info = original_data[1]

            if not isinstance(instance_info, list) or not instance_info:
                print(f"ERROR: Missing instance information for key {key}")
                continue

            # Extract node_id and function_id using regex
            match = re.search(r"node_id:\s*([\w-]+), function_id:\s*([\w-]+)", instance_info[0])
            if not match:
                print(f"ERROR: Failed to extract IDs from instance string: {instance_info[0]}")
                continue
            node_uuid, physical_uuid = match.groups()
>>>>>>> 257813b (Fix build issue)


class EDGELESSAnomalyDetectionInferer:    
    def __init__(self):
        self.config = Config()
        self.setup_logging()
        os.system('cls' if os.name == 'nt' else 'clear') if self.config.CLEAN_CLI else None
        
        self.proxy_monitor = ProxyMonitor(self.config)
        self.data_processor = DataProcessor(self.config)
        self.anomaly_detector = AnomalyDetector(self.config)
        self.running = True
        
        # Setup signal handlers for graceful shutdown
        signal.signal(signal.SIGINT, self.signal_handler)
        signal.signal(signal.SIGTERM, self.signal_handler)


    def setup_logging(self):
        log_level = logging.DEBUG if self.config.DEBUG else logging.INFO
        
        logging.basicConfig(
            level=log_level,
            format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
            handlers=[
                logging.FileHandler(
                    'outputs/logs/' + self.config.OUTPUT_EXPERIMENT_NAME + '-' +
                    datetime.now().strftime('%Y_%m_%d_%H_%M') + '.log'
                ),
                logging.StreamHandler()
            ]
        )
        
        self.logger = logging.getLogger(__name__)
    

    def signal_handler(self, signum, frame):
        self.logger.info(f"Received signal {signum}, shutting down gracefully...")
        self.running = False


    def display_debug_info(self, health_df: pd.DataFrame, performance_df: pd.DataFrame):
        """
        Display debug information about monitored data.
        
        Args:
            health_df (pd.DataFrame): Health metrics DataFrame
            performance_df (pd.DataFrame): Performance metrics DataFrame
        """     
        self.logger.debug("-"*40)
        self.logger.debug("🔍 DEBUG: MONITORED DATA")
        self.logger.debug("-"*40)
        
        self.logger.debug(f"Node Health Data:")
        if health_df.empty:
            self.logger.debug("No health data available")
        else:
            self.logger.debug(f"  Number of nodes: {health_df['node_uuid'].nunique()}")
            self.logger.debug(f"  Total records: {len(health_df)}")
            self.logger.debug(f"  Time range: {health_df['timestamp'].min()} to {health_df['timestamp'].max()}")

        self.logger.debug(f"Performance Data:")
        if performance_df.empty:
            self.logger.debug("No performance data available")
        else:
            self.logger.debug(f"  Number of physical functions: {performance_df['physical_uuid'].nunique()}")
            self.logger.debug(f"  Total Records: {len(performance_df)}")
            self.logger.debug(f"  Time range: {performance_df['timestamp'].min()} to {performance_df['timestamp'].max()}")

        self.logger.debug("-"*40)


    def display_prediction_result(self, result: Dict[str, Any]):
        """
        Display prediction result immediately to console.
        
        Args:
            result (Dict[str, Any]): Prediction result from ML model
        """
        self.logger.info("="*60)
        self.logger.info("🤖 ANOMALY DETECTION RESULT")
        self.logger.info("="*60)
        self.logger.info(f"Timestamp: {result.get('timestamp', 'Unknown')}")
        
        if 'error' in result:
            self.logger.error(f"❌ Error: {result['error']}")
        else:
            is_anomaly = result.get('is_anomaly')
            is_anomaly = result.get('anomaly_score')

            if is_anomaly is not None:
                if is_anomaly:
                    self.logger.info("🚨 STATUS: ANOMALY DETECTED")
                else:
                    self.logger.info("✅ STATUS: NORMAL")
            
            if is_anomaly is not None:
                self.logger.info(f"📊 Anomaly Score: {result['anomaly_score']}")
            
            if result.get('features_shape'):
                self.logger.info(f"📈 Features Shape: {result['features_shape']}") 

        self.logger.info("="*60)
    

    def run_inference_loop(self):
        self.logger.info("================ STARTING INFERENCE LOOP... ================")
        
        while self.running:
            try:
                inference_start = time.time()
                
                # Get current data from monitor
                self.proxy_monitor.update_data()

                node_health_data = self.proxy_monitor.get_data("node_health")
                instance_data = self.proxy_monitor.get_data("instance")
                performance_function_execution_time_data = self.proxy_monitor.get_data("performance_function_execution_time")
                performance_function_transfer_time_data = self.proxy_monitor.get_data("performance_function_transfer_time")

                # Convert to DataFrames
                instance_df = self.data_processor.instance_data_to_dataframe(instance_data)
                node_health_df = self.data_processor.node_health_data_to_dataframe(node_health_data)
                performance_df = self.data_processor.performance_data_to_dataframe(performance_function_execution_time_data, performance_function_transfer_time_data, instance_df)
                enriched_df = self.data_processor.merge_performance_with_node_health(performance_df, node_health_df)

                # Display debug info if enabled
                self.display_debug_info(node_health_df, performance_df) if self.config.DEBUG else None
                
                # Save to CSV if enabled
                self.data_processor.save_to_csv(node_health_df, "node_health_df") if self.config.OUTPUT_WRITE_TO_CSV else None
                self.data_processor.save_to_csv(performance_df, "performance_df") if self.config.OUTPUT_WRITE_TO_CSV else None
                self.data_processor.save_to_csv(enriched_df, "enriched_df") if self.config.OUTPUT_WRITE_TO_CSV else None
                # Save to Parquet if enabled
                self.data_processor.save_to_parquet(node_health_df, "node_health_df") if self.config.OUTPUT_WRITE_TO_PARQUET else None
                self.data_processor.save_to_parquet(performance_df, "performance_df") if self.config.OUTPUT_WRITE_TO_PARQUET else None
                self.data_processor.save_to_parquet(enriched_df, "enriched_df") if self.config.OUTPUT_WRITE_TO_PARQUET else None

                # # Prepare features for ML model
                # features = self.data_processor.prepare_features(node_health_df, performance_df)
                
                # if features is not None:
                #     # Perform inference
                #     result = self.anomaly_detector.predict(features)
                    
                #     # Display result immediately
                #     self.display_prediction_result(result)
                # else:
                #     print("\n⚠️  No data available for inference")

                ###### Delete in the future ######
                example_df = pd.DataFrame({
                    "cpu_usage": [0.55, 0.32, 0.91],
                    "mem_usage": [0.70, 0.48, 0.85],
                    "net_in_kbps": [120.5, 80.2, 300.1],
                    "net_out_kbps": [115.4, 76.9, 310.2],
                    "latency_ms": [12.3, 14.5, 19.8]
                })
                result = self.anomaly_detector.predict(example_df.values)
                self.display_prediction_result(result)

                for key in ["is_anomaly", "anomaly_score"]:
                    value = result.get(key)
                    self.proxy_monitor.set_data(f"anomaly_detection:{key}", str(value) if value is not None else None)
                ##################################
                
                # Wait for next inference period
                elapsed = time.time() - inference_start
                sleep_time = max(0, self.config.AD_INFERENCE_PERIOD - elapsed)
                if sleep_time > 0:
                    time.sleep(sleep_time)
                
            except Exception as e:
                self.logger.error(f"Error in inference loop: {str(e)}")
                time.sleep(self.config.AD_INFERENCE_PERIOD)
    

    def run(self):
        os.system('cls' if os.name == 'nt' else 'clear') if self.config.CLEAN_CLI else None
        self.logger.info("Starting EDGELESS Anomaly Detection System")
        self.logger.debug(f"Configuration: {self.config}")

        try:
            # Connect to the PROXY server (Redis)
            if not self.proxy_monitor.connect():
                raise Exception(f"Failed to connect to the PROXY server at redis://{self.config.PROXY_HOST}:{self.config.PROXY_PORT}")
            
            # Show orchestration domain ID
            domain_id = self.proxy_monitor.get_domain_id()
            if domain_id:
                self.logger.info(f"Orchestration Domain ID: {domain_id}")
            else:
                self.logger.warning("Could not retrieve Orchestration Domain ID key from the PROXY server")
            
            # Run inference loop
            self.run_inference_loop()

        except KeyboardInterrupt:
            self.logger.info("Received keyboard interrupt, shutting down...")
        except Exception as e:
            self.logger.error(f"Fatal error: {str(e)}")
            raise
        finally:
            self.running = False
            self.logger.info("EDGELESS Anomaly Detection Inferer stopped")
        

def main():
    try:
        system = EDGELESSAnomalyDetectionInferer()
        system.run()
    except Exception as e:
        logging.error(f"Failed to start system: {str(e)}")
        sys.exit(1)


if __name__ == "__main__":
    main()
