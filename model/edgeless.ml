(** Entity identifiers. *)

type nid = int (* Nodes *)
type wid = int (* Workflows *)
type fid = int (* Instantiated functions*)
type rid = string (* Repository identified *)
type alias = string (* Function alias within a workflow *)
type f_name = string (* Function name *)
type w_name = string (* Workflow name *)
type version = string (* Version number/name *)

(** Forwarding tables.

A forwarding table is a list of forwarding table entries associated with
a function Instance, each of which maps a specific output to one or more
function instances on other nodes with some priority for load balancing.
*)

type prio = int (* Entry priority, for load balancing *)
type output = string (* Output channel for a function *)
type target = nid * fid (* Target for a function output *)
type fwde = output * (target * prio) list
(* Forwarding table entries, the list of postential destination function
   instances with associated priorities corresponding to a function i
   nstance on this node *)

type fwdt = fwde list (* Forwarding table, a list of entries *)

(** Resources.

A node has some resources: CPU and memory are mandatory while others are
optional and may be absent.
*)

type cpu = int (* milli-vCPUs *)
type memory = int (* MB *)
type tee = Sgx | Trustzone (* Different types of TEE *)
type gpu = Cuda | Nvidia (* Different types of GPU *)
type resource = Tee of tee | Gpu of gpu (* Discrete resources *)

(** Runtimes.
    
A node has a runtime which is able to host functions of a specific
target based on the node's architecture. 
*)

type runtime = Wasm | Container | X86_64 (* What this node can host *)

(** Functions.

A function is a named entity within a function repository targeting a
particular runtime.
*)

module Function : sig
  type t
end = struct
  type t = { repository : rid; version : version; f_name : f_name; runtime : runtime; outputs : output list }
end

(** Function instance.

A function instance is a running function hosted on a node, with a
forwarding table attached.
*)

module Instance : sig
  type t
end = struct
  type t = { fid : fid; func : Function.t; fwdt : fwdt }
end

(** Function invocations.

A function invocation is a named stage in a Workflow that may invoke one
or more outputs.
*)
module Invocation : sig
  type t
end = struct
  type t = { alias : alias; func : Function.t; outputs : output list }
end

(** Workflows.
    
A workflow is a named list of function invocations, representing a single
function, chain, DAG, or general graph.
*)

module Workflow : sig
  type t
end = struct
  type t = { alias : string; functions : Invocation.t list }
end

(** Nodes.

Nodes are identified entities that use resources and a runtime to host
functions, using a forwarding table to determine where to send the
output of a function according to the workflow to which the fnuction
belongs.

A node contains the list of instances currently running.
*)
module Node : sig
  type t
end = struct
  type t = {
    nid : nid;
    resources : cpu * memory * resource list;
    runtime : runtime;
    instances : Instance.t list;
  }
end

(** Orchestrators.
    
An orchestrator -- e-orc -- is an ingress node plus a list of nodes that
can host functions. Ther eis one orchestrator per cluster of nodes.
*)

module Orchestrator : sig
  type t

  val start : t -> wid * Function.t -> t * (nid * fid) list
  val stop : t -> nid * fid -> t
  val stop : t -> wid -> t
  val update : t -> (nid * fwdt) list -> t
end = struct
  type t = Ingress of fwdt * Node.t list

  let start eorc workflow_functions = (eorc, [])
  let stop eorc node_id = eorc
  let update eorc node_forwarding_tables = eorc
end

(** Controllers.

A controller -- e-con -- is an administrative domain containing a list of
orchestrators. Controllers process workflows, making requests of
orchestrators for the functions required by a workflow to be
instantiated.
*)

module Controller : sig
  type t

  val start : t -> Workflow.t -> t * wid
  val stop : t -> wid -> t
  val list : t -> t * (wid * Orchestrator.t list)
end = struct
  type t = { Orchestrator.t list ; wid * Orchestrator.t list }

  let start econ workflow : t * wid = (econ, -1)
  let stop econ workflow_id = econ
  let list econ = econ * []
end
