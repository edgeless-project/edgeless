#!/usr/bin/env python3

import redis
import json
import time
import re
from datetime import datetime
import logging
from model_predictor import ModelPredictor

logger = logging.getLogger(__name__)

class RedisSubscriber:
    def __init__(self, config):
        """
        Initialize Redis subscriber with configuration parameters.
        
        Parameters:
        -----------
        config : dict
            Configuration dictionary containing all settings
        """
        self.config = config
        self.redis_client = redis.Redis(
            host=config['proxy_host'],
            port=config['proxy_port'],
            db=config['proxy_db'],
            decode_responses=True
        )
        self.time_window = config['time_window']
        self.model_predictor = ModelPredictor(config['model_path'])

    def get_domain_info(self):
        """Get domain ID from Redis."""
        return self.redis_client.get("domain_info:domain_id")

    def get_instances(self):
        """Get all active function instances."""
        instances = {}
        instance_keys = self.redis_client.keys("instance:*")
        for key in instance_keys:
            if key.endswith('last_update'):
                continue
            logical_uuid = key.split(':')[-1]
            if instance_data := self.redis_client.get(key):
                try:
                    data = json.loads(instance_data)
                    if "Function" in data:  # Solo nos interesan las instancias de funciones
                        instances[logical_uuid] = {
                            'data': data["Function"],
                            'node_id': None,  # Se llenará más tarde
                            'physical_id': None  # Se llenará más tarde
                        }
                        # Extraer node_id y physical_id del string de instancia
                        instance_info = data["Function"][1][0]  # Primera instancia
                        if match := re.search(r"node_id:\s*([\w-]+), function_id:\s*([\w-]+)", instance_info):
                            node_id, physical_id = match.groups()
                            instances[logical_uuid]['node_id'] = node_id
                            instances[logical_uuid]['physical_id'] = physical_id
                except (json.JSONDecodeError, KeyError, IndexError) as e:
                    logger.error(f"Error processing instance {logical_uuid}: {e}")
        return instances

    def get_metrics_in_window(self, key, window_start=None):
        """
        Get metrics from a SORTED_SET within the time window.
        
        Parameters:
        -----------
        key : str
            Redis key for the SORTED_SET
        window_start : float, optional
            Start time for the window. If None, uses current_time - time_window
        
        Returns:
        --------
        list of tuples (value, score)
            The metrics within the time window
        """
        current_time = time.time()
        start_time = window_start if window_start is not None else current_time - self.time_window
        
        return self.redis_client.zrangebyscore(
            key,
            min=start_time,
            max=current_time,
            withscores=True
        )

    def process_performance_entry(self, entry):
        """Process a performance metric entry from Redis SORTED_SET."""
        try:
            value, score = entry
            timestamp, metric_value = value.split(':')
            return {
                'value': float(metric_value),
                'timestamp': float(score)
            }
        except (ValueError, TypeError) as e:
            logger.error(f"Error processing performance entry: {e}")
            return None

    def monitor_node_health(self):
        """Monitor node health metrics from Redis SORTED_SET."""
        logger.info("\n[Node Health Status]")
        node_keys = self.redis_client.keys("node:health:*")
        for key in node_keys:
            if key.endswith('last_update'):
                continue
            node_id = key.split(':')[-1]
            latest_metrics = self.get_metrics_in_window(key)
            
            if latest_metrics:
                value, timestamp = latest_metrics[-1]  # Get most recent
                try:
                    health_data = json.loads(value)
                    logger.info(
                        f"Node {node_id:<36} | "
                        f"CPU: {health_data.get('cpu_usage', 'N/A'):>5}% | "
                        f"Memory: {health_data.get('memory_usage', 'N/A'):>5}% | "
                        f"@ {datetime.fromtimestamp(timestamp)}"
                    )
                except json.JSONDecodeError as e:
                    logger.error(f"Error decoding health data for node {node_id}: {e}")

    def monitor_function_metrics(self, logical_uuid, instance):
        """Monitor performance metrics for a specific function instance."""
        node_id = instance['node_id']
        logger.info(f"\nInstance {logical_uuid} (Node: {node_id})")

        # Get execution times
        exec_key = f"performance:{logical_uuid}:function_execution_time"
        latest_exec = self.get_metrics_in_window(exec_key)
        if latest_exec:
            if metric := self.process_performance_entry(latest_exec[-1]):
                logger.info(
                    f"  Execution time: {metric['value']:.6f}s "
                    f"@ {datetime.fromtimestamp(metric['timestamp'])}"
                )

        # Get transfer times
        transfer_key = f"performance:{logical_uuid}:function_transfer_time"
        latest_transfer = self.get_metrics_in_window(transfer_key)
        if latest_transfer:
            if metric := self.process_performance_entry(latest_transfer[-1]):
                logger.info(
                    f"  Transfer time: {metric['value']:.6f}s "
                    f"@ {datetime.fromtimestamp(metric['timestamp'])}"
                )

    def monitor_performance(self, instances):
        """Monitor performance metrics for all function instances."""
        logger.info("\n[Performance Metrics]")
        for logical_uuid, instance in instances.items():
            self.monitor_function_metrics(logical_uuid, instance)

    def get_latest_metrics(self, instance_id, node_id):
        """
        Get the latest metrics for an instance and its node.
        
        Parameters:
        -----------
        instance_id : str
            Logical UUID of the instance
        node_id : str
            ID of the node running the instance
            
        Returns:
        --------
        dict
            Dictionary with health and performance metrics
        """
        metrics = {'health': {}, 'performance': {}}
        
        # Get health metrics
        health_key = f"node:health:{node_id}"
        latest_health = self.get_metrics_in_window(health_key)
        if latest_health:
            try:
                value, _ = latest_health[-1]
                health_data = json.loads(value)
                metrics['health'].update({
                    'cpu_usage': health_data.get('cpu_usage', 0),
                    'memory_usage': health_data.get('memory_usage', 0)
                })
            except (json.JSONDecodeError, IndexError) as e:
                logger.error(f"Error processing health data: {e}")

        # Get performance metrics
        exec_key = f"performance:{instance_id}:function_execution_time"
        transfer_key = f"performance:{instance_id}:function_transfer_time"
        
        latest_exec = self.get_metrics_in_window(exec_key)
        if latest_exec and (metric := self.process_performance_entry(latest_exec[-1])):
            metrics['performance']['execution_time'] = metric['value']
        
        latest_transfer = self.get_metrics_in_window(transfer_key)
        if latest_transfer and (metric := self.process_performance_entry(latest_transfer[-1])):
            metrics['performance']['transfer_time'] = metric['value']
        
        return metrics

    def monitor_model_predictions(self, instances):
        """Run ML model predictions on current metrics."""
        logger.info("\n[ML Model Predictions]")
        
        for logical_uuid, instance in instances.items():
            node_id = instance['node_id']
            if not node_id:
                continue

            # Get latest metrics for prediction
            metrics = self.get_latest_metrics(logical_uuid, node_id)
            
            try:
                # Get prediction and confidence
                prediction, confidence = self.model_predictor.predict(metrics)
                
                # Format confidence string if available
                conf_str = f" (confidence: {confidence:.2%})" if confidence is not None else ""
                
                logger.info(
                    f"Instance {logical_uuid} (Node: {node_id}):\n"
                    f"  Prediction: {prediction}{conf_str}"
                )
            except Exception as e:
                logger.error(f"Error getting prediction for instance {logical_uuid}: {e}")

    def monitor(self):
        """
        Main monitoring loop. Shows a complete overview of the system state
        focusing on SORTED_SET metrics (performance and health status).
        """
        try:
            domain_id = self.get_domain_info()
            logger.info(f"\nStarting monitoring for Orchestration Domain: {domain_id}\n")

            while True:
                logger.info(f"\n{'='*20} Orchestration Domain: '{domain_id}' {'='*20}")

                # Get all instances and their locations
                instances = self.get_instances()
                
                # Monitor node health if enabled
                if self.config['monitor_health']:
                    self.monitor_node_health()

                # Monitor performance metrics if enabled
                if self.config['monitor_performance']:
                    self.monitor_performance(instances)

                # Run model predictions
                try:
                    self.monitor_model_predictions(instances)
                except Exception as e:
                    logger.error(f"Error running model predictions: {e}")

                logger.info("\n" + "="*70)
                time.sleep(self.config['interval'])

        except KeyboardInterrupt:
            logger.info("\nMonitoring stopped by user")
        except redis.ConnectionError:
            logger.error("Redis connection error. Is the server running?")
        except Exception as e:
            logger.error(f"Unexpected error: {e}")
            raise
