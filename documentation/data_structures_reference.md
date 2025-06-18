# Data Structures Reference

The ε-ORC in an EDGELESS orchestartion domain processes a series of data structures, which are additionally reflected in the in-memory key-value datastore.
Some of the more relevant are:

#### `NodeCapabilities`
Node *metadata* as exposed by itself to the ε-ORC when joining the *orchestration domain*. If undefined in the node's `.toml` configuration file, they are autodetected.
```json
{
    "num_cpus":16,               // sockets x cores x threads
    "model_name_cpu":"Intel(R) Xeon(R) Gold 6230 CPU @ 2.10GHz",
    "clock_freq_cpu":2095.0,     // BogoMIPS. Half the value that can be seen with lscpu or /proc/cpuinfo
    "num_cores":16,              // sockets x cores x threads
    "mem_size":32093,            // MiBs
    "labels":["hostname=edgeless-big-03"],
    "is_tee_running":false,
    "has_tpm":false,
    "runtimes":["RUST_WASM"],    // &/or "DOCKER"
    "disk_tot_space":99106,
    "num_gpus":0,
    "model_name_gpu":"",
    "mem_size_gpu":0
}
```

#### `ResourceProvider`
General metadata about a resource provider in an *orchestration domain*. They are inherently linked to the node that defined them in their `.toml` configuration file.
Field `"outputs"` contains the function's name in the workflow where the outputs/events are sent.
```json
{
    "class_type":"http-ingress",        // resource provider type
    "node_id":"18f367bb-8d21-445e-89db-6aec2bd23d7a",
    "outputs":["new_request"]           // If no output/event is sent, empty array []
}
```

#### `ActiveInstance`
This data structure is different if the logical instance is from a resource or a function.
```json
// Logical function instance
{
    "Function": [
        {
            "code": {
                "function_class_id":"ping",
                "function_class_type":"RUST_WASM",
                "function_class_version":"0.1",
                "function_class_code": [ ... ],
                "function_class_outputs": ["ponger"]
            },
            "annotations": {},
            "state_specification": {
                "state_id":"3f7ffd7b-a833-43c4-b7f0-e40690ab4b25",      // No idea
                "state_policy":"NodeLocal"
            },
            "workflow_id":"c7c1c9cf-5b38-40c2-aa35-6b298501759a"
        },
        [
            "InstanceId(node_id: ceba52c2-8465-4519-805e-fcc5e9e0ff7b, function_id: 90082fba-348a-4d06-8151-2bcf215fcd71)"       // This is a single string. Not parseable
        ]
    ]
}

// Logical Resource Instance
{
    "Resource": [
        {
            "class_type": "metrics-collector",
            "configuration": {},
            "workflow_id": "e196dc2c-a9c2-4a39-8869-e8eeabdb174f"
        },
        "InstanceId(node_id: 18f367bb-8d21-445e-89db-6aec2bd23d7a, function_id: 2167edbc-888f-4ba6-9583-1bd2431feea7)"          // This is a single string. Not parseable
  ]
}
```

#### `NodeHealthStatus`
Actualizadas cada `subscription_refresh_interval_sec`.
Basadas en las del [método USE](https://www.brendangregg.com/usemethod.html)
```json
{
    "mem_free":27105364,         // Bytes
    "mem_used":805264,           // Bytes
    "mem_available":32058476,    // Bytes
    "proc_cpu_usage":8,
    "proc_memory":23576,
    "proc_vmemory":1659944,
    "load_avg_1":4,
    "load_avg_5":2,
    "load_avg_15":12,
    "tot_rx_bytes":28449853724,
    "tot_rx_pkts":281244595,
    "tot_rx_errs":0,
    "tot_tx_bytes":430742,
    "tot_tx_pkts":174124541,
    "tot_tx_errs":0,
    "disk_free_space":103974725632,
    "disk_tot_reads":2044028928,
    "disk_tot_writes":13694341120,
    "gpu_load_perc":-1,
    "gpu_temp_cels":-1000
}
```
