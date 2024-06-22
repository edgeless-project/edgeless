### Vector multiplication example

This example shows how to create a single function or a chain of three functions performing multiplication of an internal matrix by an input vector.

First, build the `vector_mul` WASM binary following the [instructions](../../functions/README.md). 

Then, you can request the controller to start the workflow with a single function:

```
ID=$(target/debug/edgeless_cli workflow start examples/vector_mul/workflow-single.json)
```

or a chain of three functions:

```
ID=$(target/debug/edgeless_cli workflow start examples/vector_mul/workflow-chain.json)
```

Now `$ID` contains the workflow identifier assigned by the controller.

You can stop the worfklow with:

```
target/debug/edgeless_cli workflow stop $ID
```