### Fractal Demo Workflow

Calculates a mandelbrot set image and stores the rendered data in Redis.

Needs a running Redis server on default local port 6379.

The example creates the following chain:

- an HTTP ingress that waits for an external source to POST a message whose body contains calculation parameters
- a function `work_splitter` that ...
- a function `calculator` that ...
- an HTTP egress that sends the received message to an external sink

```bash
target/debug/edgeless_cli function build examples/fractal_demo/http_read_parameters/function.json
target/debug/edgeless_cli function build examples/fractal_demo/work_splitter/function.json
target/debug/edgeless_cli function build examples/fractal_demo/calculator/function.json
```

Then, you can request the controller to start the workflow:

```bash
target/debug/edgeless_cli workflow start examples/fractal_demo/workflow.json
```

In a shell use curl to emulate an external source:

```bash
# example parameter string : "1000,800,-1.2,0.35,-1.0,0.2" means
# pixel chunks of 1000x800 pixels, containing pixel data from fractal section top left: (-1.2, 0.35) to lower right: (-1.0, 0.2)
curl -v -H "Host: demo.edgeless-project.eu" http://127.0.0.1:7035/calc_fractal -d 1000,800,-1.2,0.35,-1.0,0.2
```

Non-EDGELESS application to display rendered image:
python3 gui/fractal-demo-gui.py
