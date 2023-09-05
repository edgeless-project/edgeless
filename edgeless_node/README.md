# Edgeless node

TODO: extend this with more words
Contains code for the worker node (node that executes `work` - functions - in
our EDGELESS system). Includes the agent and function runtime (for WASM).
    * Exposes the `AgentAPI` consisting of the `FunctionInstanceAPI`
    * Exposes the `InvocationAPI` (data plane)
    * Binary: `edgeless_node_d`

Also contains the RunnerAPI, State management and WASM Runtime.

TODO: any description on how the agent was implemented is welcome!