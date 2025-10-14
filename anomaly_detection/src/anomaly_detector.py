#!/usr/bin/env python3

import logging
import pickle
import numpy as np
import pandas as pd
from typing import Any, Dict
from datetime import datetime

from config import Config
from models.random_binary_model import RandomBinaryModel


class AnomalyDetector:    
    def __init__(self, config: Config):
        self.config = config
        self.logger = logging.getLogger(__name__)
        self.model = None
        self.load_model()


    def load_model(self):
        try:
            with open(self.config.EDGELESS_AD_INFERENCE_MODEL_FILE, 'rb') as file:
                self.model = pickle.load(file)

            self.logger.info(f"Anomaly Detection Model loaded successfully from {self.config.EDGELESS_AD_INFERENCE_MODEL_FILE}")
            self.logger.info(f"Model type: {type(self.model).__name__}")
            
        except FileNotFoundError:
            self.logger.error(f"Anomaly Detection Model file not found: {self.config.EDGELESS_AD_INFERENCE_MODEL_FILE}")
            raise
        except Exception as e:
            self.logger.error(f"Error loading Anomaly Detection Model: {str(e)}")
            raise
    

    def predict(self, features: pd.DataFrame | np.ndarray) -> Dict[str, Any]:
        """
        Perform anomaly detection on features.
        
        Args:
            features (np.ndarray): Feature matrix
            
        Returns:
            Dict[str, Any]: Prediction results
        """
        try:
            if self.model is None:
                raise ValueError("Model not loaded")
            
            timestamp = datetime.now()
            result = {
                'timestamp': timestamp,
                'features_shape': features.shape,
                'is_anomaly': None,
                'anomaly_score': None
            }
            
            if hasattr(self.model, 'predict'):
                prediction = self.model.predict(features)
                result['is_anomaly'] = prediction[0]
                result['anomaly_score'] = prediction[1]
            
            return result
            
        except Exception as e:
            self.logger.error(f"Error in prediction: {str(e)}")
            return {
                'timestamp': datetime.now(),
                'error': str(e),
                'is_anomaly': None,
                'anomaly_score': None
            }
