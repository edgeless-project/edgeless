# Key Value Datastore

EDGELESS uses a noSQL key-value datastore to expose and import different types of data between some of its components.
The current go-to solution is Redis, but a migration into another solution with a more OpenSource license is expected (eg. Valkey)

## How to explore the KV-datastore
To interact with the Redis server, there are thre aproaches:
- Use the `redis-cli` tool directly
- Use the utility script at `edgeless/scripts/redis_dump.sh`
- The tool you need to buld, i dont remember the name


```bash
redis-cli --scan --pattern '*' | sort
redis-cli type list
redis-cli --raw lrange <list> 0 -1 | tr '\n' ' '  # cambiar --raw con --csv

redis-cli type string
redis-cli get <string> |jq
```

### AI-based Orchestration
**DOCUMENTAR MEJOR**: Remember you can migrate a *function instance* (FID) to other node (UUID), updating two entries:
- Asign `intent:migrate:FID` a `NODE`
- Add `intent:migrate:FID` to list `intents`



## ε-ORC Data Structures


| Clave                                                 | Valor                                                                                  | Data type                              | .csv                         |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------- | -------------------------------------- | ---------------------------- |
| `domain_info:domain_id`                               | Valor **domain_id** del ε-ORC que publicó la métrica                                   | String                                 |                              |
| `node:capabilities:<node_UUID>`                       | Objeto JSON con las **capabilities** de un nodo                                        | `NodeCapabilities`                     | `capabilities.csv`           |
| `node:capabilities:last_update`                       | Timestamp unix epoch con milisegundos                                                  | String                                 |                              |
| `node:health:<node_UUID>`                             | Objeto JSON con el **health status** de un nodo.                                       | **SORTED SET**: `NodeHealthStatus`     | `health_status.csv`          |
| `provider:<provider_ID>`                              | Objeto JSON describiendo un **resource provider**                                      | `ResourceProvider`                     |                              |
| `provider:last_update`                                | Timestamp unix epoch con milisegundos                                                  | String                                 |                              |
|                                                       | **AL APLICAR UN WORKFLOW**                                                             |                                        |                              |
| `instance:<logical_UUID>`                             | Objeto JSON con información sobre una function o resource **instance**                 | `ActiveInstance`                       | `mapping_to_instance_id.csv` |
| `dependency:<logical_UUID>`                           | Objeto JSON con los outputs de una instancia, y su *<logical_UUID>*                    | `HashMap<Uuid, HashMap<String, Uuid>>` | NADA! Y sí que hace falta!!  |
| `dependency:last_update`                              | Timestamp unix epoch con milisegundos                                                  | String                                 |                              |
| `performance:function_execution_time:<physical_UUID>` | Lista de tiempos de ejecución de la función, con timestamp unich epox con milisegundos | `NodePerformanceSamples`               | `performance_samples.csv`    |

Y luego, el [[EDGELESS BENCHMARK]] me puede generar métricas adicionales `workflow:` para debugging.
Se crea el fichero **`application_metrics.csv`** con el contenido de `NodePerformanceSamples` de *Workflows*
	- `entity`: Siempre `w`, aunque debería ser `w` ó `f`.
	- `name`: Nombre del *workflow*. e.g. `wf46`
	- `value`: duración de la ejecución del *workflow*
	- `timestamp`: unix epoch en milisegundos


## Structs
#### `NodeCapabilities`
Expuestas por el nodo al ε-ORC al incorporarse al *Orchestrator Domain*. Si no se especifican en el fichero *.toml*, se autodetectan.
```json
{
    "num_cpus":16,               // sockets x cores x threads. NOTA: Probar con mas archs
    "model_name_cpu":"QEMU Virtual CPU version 2.5+",
    "clock_freq_cpu":2095.0,     // BogoMIPS. WARNING: veo el doble con lscpu ó en /proc/cpuinfo
    "num_cores":16,              // sockets x cores x threads
    "mem_size":32093,            // MiBs
    "labels":[],
    "is_tee_running":false,
    "has_tpm":false,
    "runtimes":["RUST_WASM"],    // y/o "DOCKER"
    "disk_tot_space":99158,
    "num_gpus":0,
    "model_name_gpu":"",
    "mem_size_gpu":0
}
```
El fichero **`capabilities.csv`** tiene las columnas `<additional_header>,timestamp,node_id,<NodeCapabilities>`
#### `NodeHealthStatus`
Actualizadas cada `subscription_refresh_interval_sec`.
Basadas en las del [método USE](https://www.brendangregg.com/usemethod.html)
```json
{
    "mem_free":27105364,         // Bytes
    "mem_used":805264,           // Bytes
    "mem_available":32058476,    // Bytes
    "proc_cpu_usage":8,          // ???  Punto porcentual (+ CPUs + %)
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
El fichero **`capabilities.csv`** tiene las columnas `<additional_header>,timestamp,node_id,<NodeHealthStatus>`

#### `ResourceProvider`
El campo "outputs" contiene el nombre de la función donde se va a envian los eventos y la información en cada invocación. Solo aparece en algunos resources con un valor hardcodeado.
```json
{
    "class_type":"http-ingress",        // 7 tipos
    "node_id":"18f367bb-8d21-445e-89db-6aec2bd23d7a",
    "outputs":["new_request"]           // ó []
}
```

#### `ActiveInstance`
Sintaxis rotísima
El `<logical_UUID>` asocia una función con un workflow, mientras que el `<physical_UUID>` la asocia con un nodo.
```json
// Instancia de Función
{
    "Function": [
        {
            "code": {
                "function_class_id":"ping",
                "function_class_type":"RUST_WASM",
                "function_class_version":"0.1",
                "function_class_outputs": ["ponger"]
            },
            "annotations": {},
            "state_specification": {
                "state_id":"3f7ffd7b-a833-43c4-b7f0-e40690ab4b25",      // No tengo ni idea
                "state_policy":"NodeLocal"
            },
            "workflow_id":"c7c1c9cf-5b38-40c2-aa35-6b298501759a"
        },
        [
            "InstanceId(node_id: 18f367bb-8d21-445e-89db-6aec2bd23d7a, function_id: f5149a3c-5aad-46d8-8161-359c401741f8)"
        ]
    ]
}

// Instancia de Recurso
{
    "Resource": [
        {
            "class_type": "metrics-collector",
            "configuration": {},
            "workflow_id": "e196dc2c-a9c2-4a39-8869-e8eeabdb174f"
        },
        "InstanceId(node_id: 18f367bb-8d21-445e-89db-6aec2bd23d7a, function_id: 2167edbc-888f-4ba6-9583-1bd2431feea7)"
  ]
}
```
El fichero **`mapping_to_instance_id.csv`** tiene las columnas `<additional_header>,timestamp,logical_id,node_id,physical_id`

####  `HashMap<Uuid, HashMap<String, Uuid>>`
Dependencias de cada instancia con otras.
```json
{
    "ponger":"60d247be-73c8-4117-acf1-a1e7fee0a428"
}
```

####  `NodePerformanceSamples`

| Index | Element                    |
| ----- | -------------------------- |
| 0     | 0.000776647,1739883184.785 |
| 1     | 0.000885407,1739883184.785 |
| 2     | 0.001222722,1739883186.783 |
El fichero **`performance_samples.csv`** tiene las columnas `<additional_header>,metric,identifier,value,timestamp`
`metric` siempre es `function_execution_time`, e `identifier` es el `<physical_UUID>`.




