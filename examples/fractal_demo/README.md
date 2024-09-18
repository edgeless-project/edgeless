# Fractal Demo Workflow for KPI #5.2b

This workflow is intended to become a demo scenario for EDGELESS KPI #5.2b.

`"Scenario critical tasks, as defined by Quality of Service (QoS) parameters, will be guaranteed to have less than ≤ 10 ms downtime upon (simulated) failure of any system component, thanks to hot stand-by redundant executors."`

The workflow is expected to improve our understanding of KPI-related system behavior and visualize KPI-related fault tolerance metrics.

## Demo Concept
- continuously calculate and display a varying section of some fractal (e.g. Mandelbrot set) in a distributed, fault-tolerant way
- distribute the calculation across some nodes and several function instances and introduce artificial errors (e.g. having every nth function instance fail, regularly kill some nodes, ...)
- gather and display statistics on the number of failing function instances / nodes
- measure impact of fault tolerance mechanisms and downtime

## Fractal Demo Workflow

This workflow calculates a mandelbrot set image and stores the rendered data in Redis. It requires a running Redis server which listens locally on default port 6379.

The example creates the following chain:

- an HTTP ingress that waits for an external source to POST a message whose body contains calculation parameters
- a function `work_splitter` that ...
- a function `calculator` that ...
- an HTTP egress that sends the received message to an external sink

### Build Functions

```bash
target/debug/edgeless_cli function build examples/fractal_demo/http_read_parameters/function.json
target/debug/edgeless_cli function build examples/fractal_demo/work_splitter/function.json
target/debug/edgeless_cli function build examples/fractal_demo/calculator/function.json
```

### Start Workflow

```bash
target/debug/edgeless_cli workflow start examples/fractal_demo/workflow.json
```

In a shell use curl to emulate an external source:

```bash
# example parameter string : "1000,800,-1.2,0.35,-1.0,0.2" means
# pixel chunks of 1000x800 pixels, containing pixel data from fractal section top left: (-1.2, 0.35) to lower right: (-1.0, 0.2)
curl -v -H "Host: demo.edgeless-project.eu" http://127.0.0.1:7035/calc_fractal -d 1000,800,-1.2,0.35,-1.0,0.2
```

There also is a (non-EDGELESS) python application to fetch the image data from Redis and display the rendered image:

```
python3 gui/fractal-demo-gui.py
```

### TODO

- calculate the fractal across multiple function instances / nodes
- introduce artificial faults
- gather and display data on fault tolerance metrics
- measure introduced downtime
