#!/usr/bin/env python3

import logging
import joblib
import numpy as np

logger = logging.getLogger(__name__)

class ModelPredictor:
    def __init__(self, model_path):
        """
        Initialize the model predictor.
        
        Parameters:
        -----------
        model_path : str
            Path to the pickled model file
        """
        self.model_path = model_path
        self.model = None
        self._load_model()

    def _load_model(self):
        """Load the ML model from the specified path."""
        try:
            logger.info(f"Loading model from {self.model_path}")
            self.model = joblib.load(self.model_path)
            logger.info("Model loaded successfully")
        except Exception as e:
            logger.error(f"Error loading model: {e}")
            raise

    def prepare_features(self, metrics):
        """
        Prepare features for model prediction.
        
        Parameters:
        -----------
        metrics : dict
            Dictionary containing node health and performance metrics
            
        Returns:
        --------
        array-like
            Features ready for model prediction
        """
        features = []
        
        # Extract CPU and Memory usage
        if 'health' in metrics:
            features.extend([
                metrics['health'].get('cpu_usage', 0),
                metrics['health'].get('memory_usage', 0)
            ])
        
        # Extract performance metrics
        if 'performance' in metrics:
            features.extend([
                metrics['performance'].get('execution_time', 0),
                metrics['performance'].get('transfer_time', 0)
            ])
        
        return np.array(features).reshape(1, -1)

    def predict(self, metrics):
        """
        Make a prediction using the loaded model.
        
        Parameters:
        -----------
        metrics : dict
            Dictionary containing node health and performance metrics
            
        Returns:
        --------
        prediction : object
            Model prediction result
        confidence : float
            Confidence score of the prediction (if available)
        """
        if self.model is None:
            raise RuntimeError("Model not loaded")

        try:
            # Prepare features for prediction
            features = self.prepare_features(metrics)
            
            # Make prediction
            prediction = self.model.predict(features)
            
            # Get prediction confidence if available
            confidence = None
            if hasattr(self.model, 'predict_proba'):
                probabilities = self.model.predict_proba(features)
                confidence = np.max(probabilities)
            
            return prediction[0], confidence
            
        except Exception as e:
            logger.error(f"Error making prediction: {e}")
            raise
