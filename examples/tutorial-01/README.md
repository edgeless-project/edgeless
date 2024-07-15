### tutorial-01 example

![](moving_avg.png)

First, build the `filter_in_range`, `moving_avg`, and `sensor_simulator` WASM binaries following the [instructions](../../functions/README.md). 

Then you can start and stop the workflow with:

```
ID=$(target/debug/edgeless_cli workflow start examples/tutorial-01/workflow.json)
target/debug/edgeless_cli workflow stop $ID
```