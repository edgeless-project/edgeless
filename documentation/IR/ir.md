Function Class Definition:
```json
{
    "id": "pong_async",
    "version": "0.1",
    "function_type": "RUST_WASM",
    "code": "../../functions/pong_async/pong_async.wasm",   
    "outputs": {
        "periodic_ping": {
            "method": "CAST",
            "data_type": "edgeless.example.Ping"
        }
    },
    "inputs": {
        "pong": {
            "method": "CAST",
            "data_type": "edgeless.example.Pong" 
        }
    }
}
```

Workflow Definition
```json
{
    "functions": [
        {
            "name": "ponger",
            "function_class": "pong_async:0.1",
            "output_mapping": {
                "periodic_ping": {
                    "type": "DIRECT",
                    "config": {
                        "target_component": "pinger",
                        "port": "pong"
                    }
                }
            },
            "input_mapping": {},
            "annotations": {}
        },
        {
            "name": "pinger",
            "function_class": "ping_async:0.1",
            "output_mapping": {
                "ping": {
                    "type": "DIRECT",
                    "config": {
                        "target_component": "ponger",
                        "port": "ping"
                    }
                }
            },
            "input_mapping": {},
            "annotations": {}
        }
    ],
    "resources": [],
    "annotations": {}
}
```

## Mapping Options:

### Direct:

Output mapping:
```json
"port_name": {
    "type": "DIRECT",
    "config": {
        "target_component": "function_name",
        "port": "target_port_name"
    }
}
```

OR (Multi-Target):
```json
"port_name": {
    "type": "DIRECT",
    "config": {[
        {
            "target_component": "function_name",
            "port": "target_port_name"
        },
        {
            "target_component": "function_name_2",
            "port": "target_port_name_2"
        }
    ]}
}
```


Does not require any input Mapping

### Topic-based PubSub

Output mapping:
```json
"port_name": {
    "type": "TOPIC",
    "config": {
        "topic": "/foo/bar",
        "scope": "WORKFLOW"
    }
}
```

Input mapping:
```json
"port_name": {
    "type": "TOPIC",
    "config": {
        "topic": "/foo/+",
        "scope": "WORKFLOW"
    }
}
```
### Content-Based


