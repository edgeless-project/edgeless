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

from config import Config
from proxy_monitor import ProxyMonitor
from data_processor import DataProcessor
from anomaly_detector import AnomalyDetector


class EDGELESSAnomalyDetectionInferer:    
    def __init__(self):
        self.config = Config()
        self.setup_logging()
        
        self.proxy_monitor = ProxyMonitor(self.config)
        self.data_processor = DataProcessor(self.config)
        # self.anomaly_detector = AnomalyDetector(self.config)
        
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
        self.logger.debug("\n" + "-"*40)
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
        print("\n" + "="*60)
        print("🤖 ANOMALY DETECTION RESULT")
        print("="*60)
        print(f"Timestamp: {result.get('timestamp', 'Unknown')}")
        
        if 'error' in result:
            print(f"❌ Error: {result['error']}")
        else:
            is_anomaly = result.get('is_anomaly')
            anomaly_score = result.get('anomaly_score')
            
            if is_anomaly is not None:
                if is_anomaly:
                    print("🚨 STATUS: ANOMALY DETECTED")
                else:
                    print("✅ STATUS: NORMAL")
            
            if anomaly_score is not None:
                print(f"📊 Anomaly Score: {anomaly_score}")
            
            if result.get('features_shape'):
                print(f"📈 Features Shape: {result['features_shape']}")
        
        print("="*60)
    

    def run_inference_loop(self):
        self.logger.info("=== Starting inference loop... ===")
        
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
                # NOTE: It is really complex and time-consuming to include the health data in the performance_df, so we will not do it for now.


                # Display debug info if enabled
                self.display_debug_info(node_health_df, performance_df) if self.config.DEBUG else None
                
                # Save to CSV if enabled
                self.data_processor.save_to_csv(node_health_df, "node_health_df") if self.config.OUTPUT_WRITE_TO_CSV else None
                self.data_processor.save_to_csv(performance_df, "performance_df") if self.config.OUTPUT_WRITE_TO_CSV else None
                # Save to Parquet if enabled
                self.data_processor.save_to_parquet(node_health_df, "node_health_df") if self.config.OUTPUT_WRITE_TO_PARQUET else None
                self.data_processor.save_to_parquet(performance_df, "performance_df") if self.config.OUTPUT_WRITE_TO_PARQUET else None

                # # Prepare features for ML model
                # features = self.data_processor.prepare_features(node_health_df, performance_df)
                
                # if features is not None:
                #     # Perform inference
                #     result = self.anomaly_detector.predict(features)
                    
                #     # Display result immediately
                #     self.display_prediction_result(result)
                # else:
                #     print("\n⚠️  No data available for inference")

                # Delete in the future
                is_anomaly = random.randint(0, 1)
                self.proxy_monitor.set_data("anomaly_detection:is_anomaly", str(is_anomaly))
                
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
