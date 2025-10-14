#!/usr/bin/env python3

import logging
from datetime import datetime
import signal
import threading
import time
import pandas as pd
from typing import Dict, Any
import sys

from config import Config
from proxy_monitor import ProxyMonitor
from data_processor import DataProcessor
from anomaly_detector import AnomalyDetector


class EDGELESSAnomalyDetectionInferer:    
    def __init__(self):
        self.config = Config()
        self.setup_logging()
        # os.system('cls' if os.name == 'nt' else 'clear') if self.config.CLEAN_CLI else None   # LEGACY
        
        self.proxy_monitor = ProxyMonitor(self.config)
        self.data_processor = DataProcessor(self.config)
        self.anomaly_detector = AnomalyDetector(self.config)
        
        self.stop_event = threading.Event()
        signal.signal(signal.SIGTERM, self._signal_handler)   # Termination signal
        signal.signal(signal.SIGHUP,  self._signal_handler)   # Terminal closed
        # SIGINT (Ctrl+C) left handled by KeyboardInterrupt


    def _signal_handler(self, signum, frame):
        self.stop_event.set()


    def setup_logging(self):
        log_level = logging.DEBUG if self.config.EDGELESS_AD_LOG_LEVEL_DEBUG else logging.INFO
        log_format = "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
        handlers = [logging.StreamHandler()]  # stdout

        if self.config.EDGELESS_AD_LOG_WRITE_TO_DISK:
            if self.config.EDGELESS_AD_LOG_FULLNAME:
                log_filename = f"{self.config.EDGELESS_AD_LOG_FULLNAME}.log"
            else:
                timestamp = datetime.now().strftime("%Y%m%d_%H%M")
                if self.config.EDGELESS_AD_LOG_PREFIX:
                    log_filename = f"{self.config.EDGELESS_AD_LOG_PREFIX}-{timestamp}.log"
                else:
                    log_filename = f"{timestamp}.log"

            log_file = self.config.EDGELESS_AD_LOG_DIR / log_filename
            handlers.append(logging.FileHandler(log_file))

        logging.basicConfig(
            level=log_level,
            format=log_format,
            handlers=handlers
        )
        self.logger = logging.getLogger(__name__)
    
        self.logger.debug("Logging initialized. Level=%s, WriteToDisk=%s, File=%s",
                        logging.getLevelName(log_level),
                        self.config.EDGELESS_AD_LOG_WRITE_TO_DISK,
                        log_file if self.config.EDGELESS_AD_LOG_WRITE_TO_DISK else "None")
    

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
        
        # Show data source preference and actual usage
        if self.config.EDGELESS_AD_USE_FILE_MAPPING:
            self.logger.debug("Mapping Source: FILE (with Redis fallback)")
        else:
            self.logger.debug("Mapping Source: REDIS")
        
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
        
        while not self.stop_event.is_set():
            loop_start = time.time()
            try:
                
                # Get current data from monitor
                self.proxy_monitor.update_data()
                
                node_health_data = self.proxy_monitor.get_data("node_health")
                performance_function_execution_time_data = self.proxy_monitor.get_data("performance_function_execution_time")
                performance_function_transfer_time_data = self.proxy_monitor.get_data("performance_function_transfer_time")

                # Process data into DataFrames
                instance_df = pd.DataFrame()  # Initialize empty DataFrame
                
                # Try file-based mapping first if configured
                if self.config.EDGELESS_AD_USE_FILE_MAPPING:
                    instance_df = self.data_processor.load_physical_logical_mapping_from_file(
                        self.config.EDGELESS_AD_MAPPING_FILE_PATH
                    )
                    
                    if instance_df.empty:
                        self.logger.warning("No mapping data available from file, falling back to Redis")
                    else:
                        self.logger.debug(f"Successfully loaded {len(instance_df)} records from file")
                
                # Fall back to Redis if file mapping failed or wasn't configured
                if instance_df.empty:
                    self.logger.debug("Using Redis-based instance data")
                    instance_data = self.proxy_monitor.get_data("instance")
                    instance_df = self.data_processor.instance_data_to_dataframe(instance_data)

                node_health_df = self.data_processor.node_health_data_to_dataframe(node_health_data)
                performance_df = self.data_processor.performance_data_to_dataframe(performance_function_execution_time_data, performance_function_transfer_time_data, instance_df)
                enriched_df    = self.data_processor.merge_performance_with_node_health(performance_df, node_health_df)

                # Display debug info if enabled
                self.display_debug_info(node_health_df, performance_df) if self.config.EDGELESS_AD_LOG_LEVEL_DEBUG else None
                
                # Save to CSV if enabled
                if self.config.EDGELESS_AD_DF_WRITE_TO_CSV:
                    self.data_processor.save_to_csv(node_health_df, "node_health_df")
                    self.data_processor.save_to_csv(performance_df, "performance_df")
                    self.data_processor.save_to_csv(enriched_df, "enriched_df")
                # Save to Parquet if enabled
                if self.config.EDGELESS_AD_DF_WRITE_TO_PARQUET:
                    self.data_processor.save_to_parquet(node_health_df, "node_health_df")
                    self.data_processor.save_to_parquet(performance_df, "performance_df")
                    self.data_processor.save_to_parquet(enriched_df, "enriched_df")

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
                if not self.config.EDGELESS_AD_INFERENCE_DISABLE:
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

                    # # If is_anomaly was true and EDGELESS_AD_INFERENCE_STOP_WHEN_FOUND is True, stop the application
                    # if self.config.EDGELESS_AD_INFERENCE_STOP_WHEN_FOUND and result.get("is_anomaly"):
                    #     self.logger.warning("Anomaly detected. Stopping the application.")

                ##################################
                
                # Wait for next inference period
                elapsed = time.time() - loop_start
                wait_time = max(0, self.config.EDGELESS_AD_INFERENCE_PERIOD - elapsed)
                self.stop_event.wait(wait_time)
                
            except Exception as e:
                self.logger.error(f"Error in inference loop: {str(e)}")
                self.stop_event.wait(self.config.EDGELESS_AD_INFERENCE_PERIOD)    # Wait before retrying
    

    def run(self):
        # os.system('cls' if os.name == 'nt' else 'clear') if self.config.CLEAN_CLI else None   # LEGACY
        self.logger.info("Starting EDGELESS Anomaly Detection System")
        self.logger.debug(f"Configuration: {self.config}")

        try:
            # Connect to the PROXY server (Redis)
            if not self.proxy_monitor.connect():
                raise Exception(
                    f"Failed to connect to the PROXY server at "
                    f"redis://{self.config.EDGELESS_AD_PROXY_HOST}:{self.config.EDGELESS_AD_PROXY_PORT}"
                )
            
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
            try:
                self.proxy_monitor.close()
            except Exception:
                pass
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
