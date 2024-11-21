import redis
import time
import json
import random
import pandas as pd

# DataSet columns
columns = [
    "mem_free", "mem_used", "mem_available", "proc_cpu_usage", "proc_memory",
    "proc_vmemory", "load_avg_1", "load_avg_5", "load_avg_15", "tot_rx_bytes",
    "tot_rx_pkts", "tot_rx_errs", "tot_tx_bytes", "tot_tx_pkts", "tot_tx_errs",
    "disk_free_space", "disk_tot_reads", "disk_tot_writes", "gpu_load_perc",
    "gpu_temp_cels", "node_uuid", "timestamp"
]
dataset = pd.DataFrame(columns=columns)

def extract_uuid(key):
    try:
        return key.split(":")[-1]
    except IndexError:
        return None

def main():
    ### Variable definition
    redis_host = "10.95.82.178"
    redis_port = 6379
    redis_db = 0

    global dataset
    
    redis_client = redis.StrictRedis(host=redis_host, port=redis_port, db=redis_db, decode_responses=True)
    
    try:
        print("Monigoring node health keys: 'node:health:*'")

        while True:
            health_keys = redis_client.keys("node:health:*")
            if not health_keys:
                print("WARNING: No node health keys were found")
            else:
                for key in health_keys:
                    node_health = redis_client.get(key)

                    if node_health:
                        try:
                            parsed_value = json.loads(node_health)
                            node_uuid = extract_uuid(key)
                            timestamp = round(time.time(), 3)   # rounded to miliseconds
                            
                            # Add new row
                            row = {col: parsed_value.get(col, None) for col in columns[:-2]}  # Withour columns "node_uuid" and "timestamp"
                            row["node_uuid"] = node_uuid
                            row["timestamp"] = timestamp
                            dataset = pd.concat([dataset, pd.DataFrame([row])], ignore_index=True)
                            print(f"New row added:\n{row}")

                        except json.JSONDecodeError:
                            print(f"Key: {key}")
                            print("Unparsable value:")
                            print(node_health)

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
        print("\nFinal dataset:")
        print(dataset)
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    main()

