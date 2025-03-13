import time
import json
import random
import os
import redis
import pandas as pd
import schedule

### Definition of environmen definition variables
LOOP_PERIOD = 2
REDIS_HOST = "10.95.82.178"
REDIS_PORT = 6379
REDIS_DB = 0
TIME_WINDOW = 60            # metrics collected up to TIME_WINDOW seconds old
MONITOR_NODE_HEALTH = True
MONITOR_FUNCTION_PERFORMANCE = True
CLEAN_CLI = True

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
    "timestamp", "physical_uuid", "type", "duration"
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


### Fetch metrics from Redis and generate the dataFrame for node health
def get_df_node_health(redis_client):

    node_health_dataframe = pd.DataFrame({col: [] for col in node_health_columns})

    print("Scraping node health keys: 'node:health:*'")
    if not (node_health_keys := redis_client.keys("node:health:*")):
        print("ERROR: No node health keys found. Exiting script.")
        exit(1)
    present_timestamp = round(time.time(), 3)   # rounded to miliseconds

    # Iterate for each node
    for node in node_health_keys:
        node_entries = []
        node_uuid = extract_uuid(node)
        node_health = redis_client.zrangebyscore(node, (present_timestamp - TIME_WINDOW), present_timestamp, withscores=True)

        for entry, timestamp in node_health:
            parsed_metrics = json.loads(entry)
            row = {"timestamp": timestamp, "node_uuid": node_uuid}
            row.update({col: parsed_metrics.get(col, None) for col in node_health_columns if col not in ["timestamp", "node_uuid"]})
            node_entries.append(row)

        node_health_dataframe = pd.concat([node_health_dataframe, pd.DataFrame(node_entries)], ignore_index=True)

    print("The node_health DataFrame of this iteration is:")
    print(node_health_dataframe)
    return node_health_dataframe


### Fetch metrics from Redis and generate the dataFrame for function performance
def get_df_function_performance(redis_client):

    function_performance_dataframe = pd.DataFrame({col: [] for col in function_performance_columns})

    print("Scraping function performance keys: 'performance:function_execution_time:*' and 'performance:function_transfer_time:*'")
    execution_time_keys = redis_client.keys("performance:function_execution_time:*")
    transfer_time_keys = redis_client.keys("performance:function_transfer_time:*")
    if not execution_time_keys or not transfer_time_keys:
        print("WARNING: No function performance keys found. Ignoring...")
        return function_performance_dataframe
    present_timestamp = round(time.time(), 3)   # rounded to miliseconds

    # Iterate the function execution times
    for function in execution_time_keys:
        execution_entries = []
        function_uuid = extract_uuid(function)
        execution_times = redis_client.zrangebyscore(function, (present_timestamp - TIME_WINDOW), present_timestamp, withscores=True)

        for entry, timestamp in execution_times:
            duration = function_performance_parse(entry)
            row = {
                "timestamp": timestamp,
                "physical_uuid": function_uuid,
                "type": "execution_time",
                "duration": duration
            }
            execution_entries.append(row)
        
        function_performance_dataframe = pd.concat([function_performance_dataframe, pd.DataFrame(execution_entries)], ignore_index=True)

    # Iterate the function transfer times
    for function in transfer_time_keys:
        transfer_entries = []
        function_uuid = extract_uuid(function)
        transfer_times = redis_client.zrangebyscore(function, (present_timestamp - TIME_WINDOW), present_timestamp, withscores=True)

        for entry, timestamp in transfer_times:
            duration = function_performance_parse(entry)
            row = {
                "timestamp": timestamp,
                "physical_uuid": function_uuid,
                "type": "transfer_time",
                "duration": duration
            }
            transfer_entries.append(row)

        function_performance_dataframe = pd.concat([function_performance_dataframe, pd.DataFrame(transfer_entries)], ignore_index=True)

    print("The function_performance DataFrame of this iteration is:")
    print(function_performance_dataframe)
    return function_performance_dataframe


def anomaly_detection(redis_client, df_node_health, df_function_performance):

    #############################
    #### AD logic goes here #####
    #############################

    anomaly_value = random.randint(0, 1)
    redis_client.set("anomaly_detection:anomaly", anomaly_value)
    print(f"\nKey 'anomaly_detected' updated with value: {anomaly_value}")
    print("\nWait 2 seconds...\n")


def loop_function(redis_client):
 
    os.system('clear') if CLEAN_CLI else None

    df_node_health = get_df_node_health(redis_client) if MONITOR_NODE_HEALTH else None
    df_function_performance = get_df_function_performance(redis_client) if MONITOR_FUNCTION_PERFORMANCE else None

    anomaly_detection(redis_client, df_node_health, df_function_performance)


def main():
    try:
        # Stablish a client connection with Redis
        redis_client = redis.StrictRedis(host=REDIS_HOST, port=REDIS_PORT, db=REDIS_DB, decode_responses=True)
    except Exception as e:
        print(f"ERROR: Anomaly Detection was not able to connecto to Redis: {e}")
        exit(1)

    # Schedule the loop function
    schedule.every(LOOP_PERIOD).seconds.do(loop_function, redis_client)

    print("\nStart monitoring...")      
    try:
        while True:
            schedule.run_pending()
    except KeyboardInterrupt:
        print("\nFinished monitoring...")
    except Exception as e:
        print(f"Unexpected error: {e}")
        exit(1)

if __name__ == "__main__":
    main()