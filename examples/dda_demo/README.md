# Edgeless Data Distribution Agent (DDA) - Demo

## Objective of the demo

## Prerequisites

Please perform the following step to prepare the demo:

1. Configure and build the EDGELESS framework
1. Setup the DDA environment for EDGELESS
1. Build the Demo functions

### Edgeless - Build, configure and run the framework

#### Build EDGELESS

Make sure you have build all artifacts and binaries from the EDGELESS core. For details on doing this for the different platforms, check the documentation here - [building guide](../BUILDING.md).

After completing this you should have the following executables in the directory `target/debug/` from the repository root:

| Executable name   |
| ----------------- |
| `edgeless_con_d`  |
| `edgeless_orc_d`  |
| `edgeless_node_d` |
| `edgeless_cli`    |

#### Configure EDGELESS

As next step configure the EDGELESS framework. For simplification you can run the `create_edgeless_configs.sh` script in the `scripts` folder to create standard configurations for all necessary components.

#### Run EDGELESS with one node

Next run as minimal setup of EDGELESS.
For convenience you can run the `run_edgeless_simple.sh` script in the `scripts` folder to run an Edgless Controoler, Orchestrator, and one EDGELESS node.

Via the EDGELESS cli you can now deploy workflows and functions.

#### Build the demo functions

In the `scripts` folder you can find two functions implemented in rust:

1. `check_temperature` - TODO what does it do
1. `move_arm` - - TODO what does it do

You can build the functions by calling `../../../target/debug/edgeless_cli function build ../functions/check_temperature/function.json` for the check temperature function and `../../../target/debug/edgeless_cli function build ../functions/move_arm/function.json` respectively for the move_arm function.

For convenience you can simply call `build_demo_functions.sh`in the `scripts` folder.

### Data Distribution Agent (DDA) - Setup the DDA environment for EDGELESS

1. Run MQTT Broker
1. Configure MQTT Broker in DDA `dda.yaml` and mock functions. 

## Run the demo

1. deploy and run workflow with `UUID=$(../../../target/debug/edgeless_cli workflow start ../workflow.json)` or call  `run_demo_workflow.sh` in the `scripts`folder.
1. run mock functions:
    1. First run an actor `run_mock_actor.sh` in `scripts` folder
    1. Second run a sensor `run_mock_sensor.sh` in `scripts` folder

You should see A, B, C

## License

Code and documentation copyright 2024 Siemens AG.

Code is licensed under the [MIT License](https://opensource.org/licenses/MIT).

Documentation is licensed under a
[Creative Commons Attribution-ShareAlike 4.0 International License](http://creativecommons.org/licenses/by-sa/4.0/).
