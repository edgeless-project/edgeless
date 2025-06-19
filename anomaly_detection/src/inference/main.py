#!/usr/bin/env python3

import logging
from config import get_config
from redis_subscriber import RedisSubscriber

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

def main():
    """
    Main entry point for the EDGELESS Anomaly Detection Inferer.
    Initializes configuration and starts the monitoring process.
    """
    try:
        # Get configuration from environment variables and command line arguments
        config = get_config()
        
        # Initialize and start the Redis subscriber
        subscriber = RedisSubscriber(config)
        subscriber.monitor()
    except KeyboardInterrupt:
        logger.info("\nProgram terminated by user")
    except Exception as e:
        logger.error(f"Fatal error: {e}")
        raise

if __name__ == "__main__":
    main()
