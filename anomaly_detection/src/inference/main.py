#!/usr/bin/env python3

import logging
from datetime import datetime
import signal
import os
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
        
        self.proxy_monitor = ProxyMonitor(self.config)
        self.data_processor = DataProcessor(self.config)
        self.anomaly_detector = AnomalyDetector(self.config)
        
        self.running = True
        self.monitor_thread = None
        
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
        print("\n" + "-"*40)
        print("🔍 DEBUG: MONITORED DATA")
        print("-"*40)
        
        print(f"Health Data:")
        if health_df.empty:
            print("  No health data available")
        else:
            print(f"  Records: {len(health_df)}")
            print(f"  Unique keys: {health_df['key'].nunique()}")
            print(f"  Time range: {health_df['datetime'].min()} to {health_df['datetime'].max()}")
            print(f"  Sample keys: {list(health_df['key'].unique()[:3])}")
        
        print(f"\nPerformance Data:")
        if performance_df.empty:
            print("  No performance data available")
        else:
            print(f"  Records: {len(performance_df)}")
            print(f"  Unique keys: {performance_df['key'].nunique()}")
            print(f"  Time range: {performance_df['datetime'].min()} to {performance_df['datetime'].max()}")
            print(f"  Sample keys: {list(performance_df['key'].unique()[:3])}")
        
        print("-"*40)


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
                health_data, performance_data = self.proxy_monitor.get_current_data()
                
                # Convert to DataFrames
                health_df = self.data_processor.redis_data_to_dataframe(health_data, "health")
                performance_df = self.data_processor.redis_data_to_dataframe(performance_data, "performance")
                
                # Display debug info if enabled
                self.display_debug_info(health_df, performance_df) if self.config.DEBUG else None
                
                # Save to CSV if enabled
                self.data_processor.save_to_csv(health_df, performance_df) if self.config.OUTPUT_WRITE_TO_CSV else None
                
                # # Prepare features for ML model
                # features = self.data_processor.prepare_features(health_df, performance_df)
                
                # if features is not None:
                #     # Perform inference
                #     result = self.anomaly_detector.predict(features)
                    
                #     # Display result immediately
                #     self.display_prediction_result(result)
                # else:
                #     print("\n⚠️  No data available for inference")
                
                # Wait for next inference period
                elapsed = time.time() - inference_start
                sleep_time = max(0, self.config.AD_INFERENCE_PERIOD - elapsed)
                if sleep_time > 0:
                    time.sleep(sleep_time)
                
            except Exception as e:
                self.logger.error(f"Error in inference loop: {str(e)}")
                time.sleep(self.config.AD_INFERENCE_PERIOD)
    

    def run(self):
        self.logger.info("Starting EDGELESS Anomaly Detection System")
        self.logger.debug(f"Configuration: {self.config}")
        
        # Clear console
        os.system('cls' if os.name == 'nt' else 'clear') if self.config.CLEAN_CLI else None
        
        try:
            # Connect to the PROXY server (Redis)
            if not self.proxy_monitor.connect():
                raise Exception(f"Failed to connect to the PROXY server at redis://{self.config.PROXY_HOST}:{self.config.PROXY_PORT}")
            
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
