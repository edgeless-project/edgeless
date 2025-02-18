import redis
import time
import json
import random
import pandas as pd

### Variable definition
REDIS_HOST = "10.95.82.178"
REDIS_PORT = 6379
REDIS_DB = 0
MONITOR_NODE_HEALTH = True
MONITOR_FUNCTION_PERFORMANCE = True

node_health_columns = [
    "timestamp", "node_uuid", "mem_free", "mem_used", "mem_available",
    "proc_cpu_usage", "proc_memory","proc_vmemory", "load_avg_1", "load_avg_5", "load_avg_15",
    "tot_rx_bytes", "tot_rx_pkts", "tot_rx_errs", "tot_tx_bytes", "tot_tx_pkts", "tot_tx_errs",
    "disk_free_space", "disk_tot_reads", "disk_tot_writes", "gpu_load_perc","gpu_temp_cels"
]
function_performance_columns = [
    "timestamp", "physical_uuid", "execution_time"
]


def extract_uuid(key):
    try:
        return key.split(":")[-1]
    except IndexError:
        return None

def execution_time_parse(entry):
    try:
        execution_time, timestamp = map(float, entry.split(","))
        return execution_time, timestamp
    except ValueError:
        return None, None


def main():

    try:
        redis_client = redis.StrictRedis(host=REDIS_HOST, port=REDIS_PORT, db=REDIS_DB, decode_responses=True)
    except Exception as e:
        print(f"ERROR: Anomaly Detection was not able to connecto to Redis: {e}")
        exit(1)

    if MONITOR_NODE_HEALTH:
        print("Monitoring node health keys: 'node:health:*'")
        node_health_dataset = pd.DataFrame(columns=node_health_columns)
    if MONITOR_FUNCTION_PERFORMANCE:
        print("Monitoring function performance keys: 'performance:function_execution_time:*'")
        function_performance_dataset = pd.DataFrame(columns=function_performance_columns)
        latest_timestamps = {}  # Saves the latest timestamp of each function.


    while True:
        try:
            if MONITOR_NODE_HEALTH:
                node_health_keys = redis_client.keys("node:health:*")

                if not node_health_keys:
                    print("ERROR: No node health keys were found")
                    exit(1)

                for key in node_health_keys:
                    node_health = redis_client.get(key)

                    try:
                        parsed_value = json.loads(node_health)
                        node_uuid = extract_uuid(key)
                        timestamp = round(time.time(), 3)   # rounded to miliseconds

                        row = {col: parsed_value.get(col, None) for col in node_health_columns}
                        row["timestamp"] = timestamp
                        row["node_uuid"] = node_uuid

                        node_health_dataset = pd.concat([node_health_dataset, pd.DataFrame([row])], ignore_index=True)
                        print(f"New row added:\n{row}")

                    except json.JSONDecodeError:
                        print(f"ERROR: Key {key} contains an unparsable JSON value: {node_health}")


            if MONITOR_FUNCTION_PERFORMANCE:
                function_performance_keys = redis_client.keys("performance:function_execution_time:*")    
                new_entries = []

                if function_performance_keys:
                    for key in function_performance_keys:
                        physical_uuid = extract_uuid(key)
                        function_performance = redis_client.lrange(key, 0, -1)

                        for entry in function_performance:
                            execution_time, timestamp = execution_time_parse(entry)

                            if timestamp > latest_timestamps.get(physical_uuid, 0):
                                new_entries.append([timestamp, physical_uuid, execution_time])
                                latest_timestamps[physical_uuid] = timestamp


                if new_entries:
                    new_function_performance_dataframe = pd.DataFrame(new_entries, columns=function_performance_columns)
                    function_performance_dataset = pd.concat([function_performance_dataset, new_function_performance_dataframe], ignore_index=True)
                    function_performance_dataset.sort_values(by=["timestamp"], inplace=True)
                    print(f"New entries added:\n{new_function_performance_dataframe}")


                #############################
                #### AD logic goes here #####
                #############################

                anomaly_value = random.randint(0, 1)
                redis_client.set("anomaly_detection:anomaly", anomaly_value)
                print(f"Key 'anomaly_detected' updated with value: {anomaly_value}")
                print("\nWait 2 seconds...\n")
                time.sleep(2)

        except KeyboardInterrupt:
            print("\nFinished monitoring...")
            if MONITOR_NODE_HEALTH:
                print("\nFinal node_health dataset:")
                print(node_health_dataset)
            if MONITOR_FUNCTION_PERFORMANCE:
                print("\nFinal function_performance dataset:")
                print(function_performance_dataset)
            break
        except Exception as e:
            print(f"Unexpected error: {e}")
            exit(1)

if __name__ == "__main__":
    main()
