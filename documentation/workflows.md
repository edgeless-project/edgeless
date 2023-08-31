# Workflow Definitions

From a workflow-developer perspective, workflows can be defined using a
`workflow.json` file. Internally, the `edgeless_cli` uses the
`WorkflowInstanceAPI` to start/stop workflows. As they are rather similar, we
will focus on the `workflow.json` format here. Take a look at an example
workflow configuration (don't worry, we will explain what the different names
mean in more detail further down).

## Example Workflow Instance Configuration
```json
{
    "alias": "workflow_alias",
    "functions": [
        {
            "alias": "stage_1_function", // this alias is scoped inside of this workflow and may be undefined outside of it
            "class_specification": { // this property uniquely identifies a function
                "id": "http_processor",
                "function_type": "RUST_WASM",
                "version": "0.1",
                "include_code_file": "./compiled_function.wasm",
                // TODO: why does this need to be inside of the class_specification? Why can't we move it outside
                // if there is a reason, document it
                "output_callbacks": ["success_cb", "failure_cb"] 
            },
            "output_callback_definitions": { // this section specifies the mapping between 
            // TODO: rename to mapping? / something else
                "success_cb": "http_processor_stage_2",
                "failure_cb":
            },
            "annotations": {}
        },
        {
            "alias": "http_processor_stage_2",
            "class_specification": {
                "id": "http_processor2",
                "function_type": "RUST_WASM",
                "version": "0.1",
                "include_code_file": "./processing_function2/http_processor2.wasm",
                "output_callbacks": []
            },
            "output_callback_definitions": {},
            "annotations": {}
        },
    ],
    "resources": [
        {
            "alias": "http-ingress-1-1",
            "resource_class_type": "http-ingress",
            "output_callback_definitions": {
                // specified where the new_requests to the ingress should go to
                "new_request": "http_processor_stage_1" 
            },
            "configurations": {
                "host": "demo.edgeless.com",
                "methods": "POST"
            }
        }
    ],
    "annotations": {}
}
```

A Workflow Instance Configuration as the one shown above contains four main
elements:

* The `alias`, which should be unique within a namespace (TODO: define what this
  means - what namespace are you referring to).
* The list of function instance definitions (`functions`).
    *   In a later version of edgeless that contains scaling, each of these
        items might need to point to multiple function instances.
* The list of resource instance definitions (`resources`).
* The workflow annotations.


A Function Instance Definition (two of them are shown in the example above)
contains the following elements:

* The `alias` identifies the function instance within the scope of this
  workflow.
* Information about the function class (`class_specification`):
    *   In any case, this needs to uniquely identify the function class. This
        can be achieved by the combination of an `id`, `version`, and
        `function_type`. Analogous to docker containers, this can be used to
        fetch the function from a repository.
    *   In the current prototype, the `class_specification` also contains a link
        to the function code and additional metadata specific to the function
        class:
        * `output_callbacks` specifies which outputs the function class will use
          in `call_alias`/`cast_alias` calls. Those need to be mapped to other
          functions for the events to be used. TODO: this is very fuzzy
* The mapping of the callbacks defined in the class specification to other
  functions/resources defined in this workflow (identified by the `alias`).
* The function annotations.


A Resource Instance Definition (one of them is shown above) contains the
following elements:

* The `alias` identifies the resource instance within this workflow.
* The `resource_class_type` defines which type of resource must be instantiated.
  Compared to the functions that are not known to the controller unless
  instantiated, the resource providers register themselves and their resource
  classes directly with the controller.
* The mapping of the output callbacks to functions/resources in the workflow
  (`output_callback_definitions`). This is analogous to the functions.
* The configuration of the resource instance (`configurations`). This is used
  to, e.g., configure the webserver to accept the desired requests that are then
  routed towards the functions.

The example above shows a workflow consisting of an http-ingress and two
processing functions. When a new request matching the configuration is received
by the ingress, it sends an event to the `http_processor_stage_1` function as
this is configured in the (http-ingresses') resource's
`output_callback_definitions` (specified under `new_request`). This function
subsequently sends a specific set of events (`success_cb`) towards the
`http_processor_stage_2` function.

