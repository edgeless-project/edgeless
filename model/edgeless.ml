(** Entity identifiers. *)

type nid = int (* Nodes *)
type wid = int (* Workflows *)
type fid = int (* Functions*)

(** Forwarding tables.

A forwarding table is a list of forwarding table entries, each of which
maps a specific function instance in a specific workflow to one or more
function instances on other nodes with some priority for load balancing.
*)

type prio = int (* Entry priority, for load balancing *)
type fwde = (wid * fid) * ((nid * fid) * prio) list
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
    
A function is an identified entity within a function repository.
*)

module Function : sig
  type t
end = struct
  type t = {
    fid : fid;
    runtime : runtime;
    repository : string;
  }
end


(** Workflows.
    
A workflow is an identified sequence of function calls.
*)

module Workflow : sig
  type t
end = struct
  type t = {
    wid : wid;
    chain : Function.t list;
  }
end

(** Nodes.

Nodes are identified entities that use resources and a runtime to host
functions, using a forwarding table to determine where to send the
output of a function according to the workflow to which the fnuction
belongs.
*)
module Node : sig
  type t
end = struct
  type t = {
    nid : nid;
    resources : cpu * memory * resource list;
    runtime : runtime;
    fwdt : fwdt;
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
  val update : t -> (nid * fwdt) list -> t
end = struct
  type t = Ingress of fwdt * Node.t list

  let start eorc workflow_functions = (eorc, [])
  let stop eorc node_id = eorc
  let update eorc node_forwarding_tables = eorc
end

(** Controllers.

A controller -- e-con -- is an administrative domain contaiing a list of
orchestrators. Controllers process workflows, making requests of
orchestrators for the fnuctions required by a workflow to be
instantiated.
*)

module Controller : sig
  type t

  val start : t -> Workflow.t -> t * wid
  val stop : t -> wid -> t
end = struct
  type t = Orchestrator.t list

  let start econ workflow = (econ, -1)
  let stop econ workflow_id = econ
end
