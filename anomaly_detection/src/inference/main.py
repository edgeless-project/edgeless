#!/usr/bin/env python3

import logging
from datetime import datetime
import signal
import os
import time
import pandas as pd
...
import sys

from config import Config
from proxy_monitor import ProxyMonitor
from data_processor import DataProcessor
from anomaly_detector import AnomalyDetector


# import pickle
# import redis


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
    

    def signal_handler(self, signum):
        self.logger.info(f"Received signal {signum}, shutting down gracefully...")
        self.running = False


    def display_debug_info(self, health_df: pd.DataFrame, performance_df: pd.DataFrame):
        """
        Display debug information about monitored data.
        
        Args:
            health_df (pd.DataFrame): Health metrics DataFrame
            performance_df (pd.DataFrame): Performance metrics DataFrame
        """
        if not self.config.DEBUG:
            return
        
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
                self.display_debug_info(health_df, performance_df)
                
                # Save to CSV if enabled
                timestamp_str = datetime.now().strftime("%Y%m%d_%H%M%S")
                self.data_processor.save_to_csv(health_df, performance_df, timestamp_str)
                
                # Prepare features for ML model
                features = self.data_processor.prepare_features(health_df, performance_df)
                
                if features is not None:
                    # Perform inference
                    result = self.anomaly_detector.predict(features)
                    
                    # Display result immediately
                    self.display_prediction_result(result)
                else:
                    print("\n⚠️  No data available for inference")
                
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



# # Fetch sorted set keys for a given pattern
# def get_sorted_set_keys(r, pattern):
#     return [k for k in redis_client.scan_iter(match=pattern)]

# # Fetch data from sorted sets within a time window
# def fetch_metrics(r, keys, time_window):
#     now = int(time.time())
#     min_score = now - time_window
#     data = []
#     for key in keys:
#         values = redis_client.zrangebyscore(key, min_score, now, withscores=True)
#         for value, score in values:
#             data.append({'key': key, 'value': value, 'timestamp': int(score)})
#     return pd.DataFrame(data)

# # Optionally write dataframe to CSV
# def write_dataframe_to_csv(df, metric_type):
#     if not os.path.exists(f'outputs/{OUTPUT_EXPERIMENT_NAME}'):
#         os.makedirs(f'outputs/{OUTPUT_EXPERIMENT_NAME}')
#     filename = f'outputs/{OUTPUT_EXPERIMENT_NAME}/{metric_type}_{datetime.now().strftime("%Y%m%d_%H%M%S")}.csv'
#     df.to_csv(filename, index=False, header=OUTPUT_COLUMNS)

    

    # logger.info("\n=== Starting Anomaly Detection... ===")
    # detector.run_continuous_detection(
    #     data_generator=example_data_generator(),
    #     interval=2.0,
    #     max_iterations=20
    # )

    # while True:
    #     os.system('clear') if CLEAN_CLI else None 

    #     # Fetch health and performance metrics
    #     health_keys = get_sorted_set_keys(r, 'health:status:*')
    #     perf_keys = get_sorted_set_keys(r, 'performance:*')
    #     health_df = fetch_metrics(r, health_keys, AD_TIME_WINDOW)
    #     perf_df = fetch_metrics(r, perf_keys, AD_TIME_WINDOW)

    #     # Optionally show dataframes
    #     if DEBUG:
    #         print("[DEBUG] Health metrics:")
    #         print(health_df)
    #         print("[DEBUG] Performance metrics:")
    #         print(perf_df)
    #     # Optionally write to CSV
    #     if OUTPUT_WRITE_TO_CSV:
    #         write_dataframe_to_csv(health_df, 'health')
    #         write_dataframe_to_csv(perf_df, 'performance')
    #     # Inference (if there is data)
    #     for df, metric_type in [(health_df, 'health'), (perf_df, 'performance')]:
    #         if not df.empty:
    #             try:
    #                 # Assume model expects a dataframe or similar structure
    #                 result = anomaly_detector.predict(df)
    #                 print(f"[INFERENCE][{metric_type}] {result}")
    #             except Exception as e:
    #                 print(f"[ERROR] Inference failed for {metric_type}: {e}")
    #     time.sleep(AD_INFERENCE_PERIOD)

if __name__ == "__main__":
    main()
