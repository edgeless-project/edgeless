# Library of example EDGELESS functions

The functions included in this directory are used by the
[examples](../examples/README.md) and can serve as a template or starting point to help you write _your_ functions.

> DISCLAIMER: The functions are provided as examples. They are not fully
> optimized and tested and, thus, they are not meant to be used in production
> services. Use at your own risk.

## How to build the functions

To build a single function, e.g., the `noop`:

1. Make sure you built the EDGELESS executables following the [instructions](building.md)
2. Run the following command from the root of the repository:

```shell
target/debug/edgeless_cli function build functions/noop/function.json
```

To build *all* the functions you can use this script:

```shell
scripts/functions_build.sh
```