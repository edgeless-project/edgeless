#This script reads metrics from a RedisDB and forwards them to an Elasticsearch Endpoint

# Required Libraries
import redis
from elasticsearch import Elasticsearch, helpers
import json
from datetime import datetime, timezone
import os
import logging


# Get environment variables
REDIS_HOST = os.getenv('REDIS_HOST', '127.0.0.1')
REDIS_PORT = int(os.getenv('REDIS_PORT', '6379'))
ELASTICSEARCH_HOST = os.getenv('ELASTICSEARCH_HOST', 'https://edgeless1.iit.cnr.it:9200')
ELASTICSEARCH_USER = os.getenv('ELASTICSEARCH_USER', 'edgeless')
ELASTICSEARCH_PASSWORD = os.getenv('ELASTICSEARCH_PASSWORD', '5T^97^QiR2?t')

# Connect to Redis
redis_client = redis.Redis(host=REDIS_HOST, port=REDIS_PORT, db=0)

# Connect to Elasticsearch
es = Elasticsearch(
    [ELASTICSEARCH_HOST],
    basic_auth=(ELASTICSEARCH_USER, ELASTICSEARCH_PASSWORD),
    verify_certs=True,
    ssl_show_warn=True
)
logging.basicConfig(
    level=logging.WARNING,  # Change to DEBUG for more details
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[
        logging.StreamHandler()  # Log to console

    ]
)

#current support indexes are: node, provider, dependency, instance, function, performance, workflow

def index_node_health(key, value):
    """
    Index a node_health entry into Elasticsearch.
    """
    global index_counter
    try:
        # Decode and parse the Redis key and value
        key_str = key.decode()
        value_str = value.decode()
        node_id = key_str.split(':')[-1]  # Extract node ID from key
        doc = json.loads(value_str)
        
        # Add additional metadata
        # doc['timestamp'] = datetime.now().isoformat()
        doc['node_id'] = node_id

        # Index the document into Elasticsearch
        index_name = "node_health_test"
        response = es.index(index=index_name, body=doc)
        index_counter += 1
        # print(f"[{index_counter}] Indexed node_health for node_id: {node_id}, result: {response['result']}")
    
    except Exception as e:
        print(f"Error indexing node_health for key {key}: {e}")



index_counter = 0
#index Redis entries into Elasticsearch
def index_to_elasticsearch(key, value):
    global index_counter
    key_type = redis_client.type(key).decode()
    print(f"Key: {key.decode()}, Type: {key_type}")
    # print(f"Value: {value.decode()}")
    key_parts = key.decode().split(':')
    index = key_parts[0]  # index type: node, provider, dependency, instance, function, performance, workflow
    timestamp = datetime.now().isoformat()

    # Default doc to None
    doc = {}
    # handle the case where value is a float (for lists type keys)
    if key_type == "string":
        try:
            if not isinstance(doc, dict):  # Ensure doc is a dictionary
                doc = {"value": doc}
        except json.JSONDecodeError:
            logging.warning(f"Value is not a JSON object: {value.decode()}")
            try:
                doc = {"value": float(value.decode())}  # Wrap float in a dictionary
            except ValueError:
                logging.error(f"Value is not a float: {value.decode()}")
                return

    elif key_type == "list":
        # Handle lists (for example, function samples)
        values = redis_client.lrange(key, 0, -1)  # Get the entire list
        doc = [
            {"value": float(item.decode().split(',')[0]), "timestamp": float(item.decode().split(',')[1])}
            for item in values
        ]
    
    if index == "domain_info":
        domain_id = value.decode()  # Ensure it's a string
        logging.debug("Domain_id:",domain_id)
        doc["domain_id"] = domain_id  # Store as a string in Elasticsearc
        index_name = "domain_info" 
        logging.debug(f"Forwarding domain entry to Elasticsearch: {doc}")
    # index based on the index type
    elif index == "function":
        function_id = key_parts[1]
        
        if key_parts[2] == "average":
            # the value is a float
            doc = {'function_id': function_id, 'average': float(value.decode()), 'timestamp': timestamp}
        
        elif key_parts[2] == "samples":
            # Handle 'samples' as a list of floats and timestamps
            doc = {
                'function_id': function_id,
                'samples': doc,  
                'timestamp': timestamp
            }
        index_name = "function"

    elif index == "node":
        node_metric_type = key_parts[1]  # "capabilities" or "health"
        node_id = key_parts[2]
        # doc['timestamp'] = timestamp
        doc['node_id'] = node_id

        if node_metric_type == "capabilities":
            index_name = "node_capabilities"
        elif node_metric_type == "health":
            index_name = "node_health_test"
        else:
            print(f"Unrecognized node metric type for key: {key.decode()}")
            return
    elif index == "provider":
        # doc['timestamp'] = timestamp
        node_id = doc.get('node_id')
        if node_id is None:
            print(f"Skipping key: {key} due to missing node_id in provider entry")
            return
        doc['node_id'] = node_id
        index_name = "provider"
    elif index == "dependency":
        doc['timestamp'] = timestamp
        doc['dependency_id'] = key_parts[1]
        index_name = "dependency"
        
    elif index == "instance":
        doc['timestamp'] = timestamp
        instance_id = key_parts[1]
        doc['instance_id'] = instance_id
        # Normalize Resource field if it exists
        if 'Resource' in doc:
            normalized_resource = []
            for item in doc['Resource']:
                if isinstance(item, dict):
                    normalized_resource.append(item)
                else:
                    normalized_resource.append({"InstanceId": item})
                doc['Resource'] = normalized_resource
        
        # Normalize Function field if it exists
        if 'Function' in doc:
            normalized_function = []
            for item in doc['Function']:
                if isinstance(item, dict):
                    normalized_function.append(item)
                else:
                    normalized_function.append({"InstanceId": item})
                doc['Function'] = normalized_function
        
        index_name = "instance"

    elif index == "performance":
        function_id = key_parts[2]
        # Handle performance data (list of execution times and timestamps)
        doc = {
            'function_id': function_id,
            'execution_times': doc,  
            'timestamp': timestamp
        }
        index_name = "performance"
    
    elif index == "workflow":
        #convert to dict if not
        if not isinstance(doc, dict):
            doc = {}
        workflow_metric_type = key_parts[1]  # e.g., vector_mul_wf_chain
        doc['workflow_type'] = workflow_metric_type

        if key_parts[2] == "average":
            # Handle average value
            doc['average'] = float(value.decode())
            

        elif key_parts[2] == "samples":
            # Handle workflow samples
            samples = value.decode().split()
            doc['samples'] = [
                {"value": float(sample.split(',')[0]), "timestamp": float(sample.split(',')[1])}
                for sample in samples
            ]
        index_name = "workflow"
    else:
        logging.warning(f"Skipping key: {key.decode()} due to unrecognized index")
        return    
    # forward into Elasticsearch
    try:
        index_counter = index_counter + 1
        res = es.index(index=index_name, body=doc)
        logging.debug(f"Indexed document: {index_counter})  {res['result']}")


        # print(f"Total indexed documents: {index_counter}")
    except Exception as e:
        logging.error(f"Error indexing document: {e}")

# on the start of the script go to redis and fetch all existing entries
def index_existing_entries():
    cursor = 0
    logging.info("Indexing existing entries in Redis")
    while True:
        cursor, keys = redis_client.scan(cursor=cursor)
        for key in keys:
            key_type = redis_client.type(key).decode()

            # Handle only string keys (type = "string")
            if key_type == "string":
                value = redis_client.get(key)
                if value:
                    index_to_elasticsearch(key, value)
            elif key_type == "list":
                values = redis_client.lrange(key, 0, -1)
                if values:
                    for value in values:
                        index_to_elasticsearch(key, value)
                # print(f"Skipping key: {key.decode()} due to incompatible type: {key_type}")
            else:
                logging.warning(f"Skipping key: {key.decode()} due to incompatible type: {key_type}")
        
        if cursor == 0:
            break

# keep listening on redis channel for new entries
def listen_for_new_entries(redis_channel):
    pubsub = redis_client.pubsub()
    pubsub.psubscribe('__key*__:*')

    logging.info(f"Subscribed to Redis channel: {redis_channel}")

    for message in pubsub.listen():
        if message['type'] == 'pmessage':
            key = message['data']
            # Decode the key and filter for node:health entries
            key_str = key.decode()
            if key_str.startswith("node:health:"):
                value = redis_client.get(key)
                if value:
                    logging.debug(f"Processing node:health key: {key_str}")
                    index_node_health(key, value)
                else:
                    logging.debug(f"Received node:health key: {key_str}, but no value found in Redis.")
            else:
                    logging.debug(f"Ignored key: {key_str}")

# Index all existing entries on script startup



def check_redis_connection():
    """Checks if Redis is reachable."""
    try:
        redis_client.ping()  # Test connection
        logging.info("✅ Redis connection successful")
    except redis.ConnectionError as e:
        logging.error(f"❌ Redis connection failed: {e}")

def check_elasticsearch_connection():
    """Checks if Elasticsearch is reachable."""
    try:
        es = Elasticsearch([ELASTICSEARCH_HOST])
        if es.ping():
            logging.info("✅ Elasticsearch connection successful")
        else:
            logging.warning("❌ Elasticsearch is unreachable")
    except Exception as e:
        logging.error(f"❌ Elasticsearch connection failed: {e}")

# Subscribe to listen for new entries
if __name__ == "__main__":
    check_redis_connection()
    check_elasticsearch_connection()
    index_existing_entries()    
    redis_channel = '__key*__:*'  # Specify the Redis channel to subscribe to
    listen_for_new_entries(redis_channel)

#    #redis keyspace notifications: redis-cli config set notify-keyspace-events K$