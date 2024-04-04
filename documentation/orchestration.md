# EDGELESS orchestration

Orchestration in EDGELESS happens at two levels:

- _higher level orchestration_ is done by the ε-CON at cluster level (remember
  that a cluster may include multiple non-overlapping orchestratio domains)
  and it maps (logical) function to orchestration domains;
- _lower level orchestration_ is done by the ε-ORC within its orchestration
  domain, and it maps every (logical) function to one or multiple function
  instances running on the orchestration domain nodes.

## Higher level orchestration

Work in progress

## Lower level orchestration

The ε-ORC implements a basic orchestration policy that:

1) honors the deployment requirements, and
2) ensures that one function instance is maintained in execution for all the
   accepted logical functions, if possible based on the deployment requirements.

If it is currently not possible to maintain in execution a function instance of
a given logical function, the ε-ORC will continue trying to create the
function instance every `keep_alive_interval_secs` (a configurable parameter
in the TOML file).

The same period is also used to poll all the nodes in an orchestration domain:
if a node does not respond, then it is immediately removed from the list of
active nodes and its functions/resources are relocated to other nodes, if
possible.

In all cases, the ε-ORC ensures that "patching", i.e., the interconnections
among function instances and resources for the exchange of events, is kept
up-to-date with the current components in execution.

Algorithms:

- If there are multiple resource providers that can host a resource,
  the ε-ORC selects one at random.
- If there are multiple nodes that can host a function instance, the ε-ORC
  uses one of the two basic strategies (which can be selected in the
  configuration file with `orchestration_strategy`):
  - `Random`: each node is assigned a weight equal to the product of the
  advertised number of CPUs, advertised number of cores per CPU, and
  advertised core frequency; then the node is selected using a weighted
  uniform random distribution;
  - `RoundRobin`: the ε-ORC keeps track of the last node used and
  assigns the next one (with wrap-around) among those eligible; note that
  this strategy does _not_ guarantee fairness if functions with different
  deployment requirements are requested.