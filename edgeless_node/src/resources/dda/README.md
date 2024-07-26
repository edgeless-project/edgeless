# DDA Integration Status

## What is working:
- function developer can use any of the functionalities of Com, State and Store
- for two-way functions, only the first response is returned
- all incoming events to the DDA resource are processes sequentially - by design
- no guarantees that the dataplane won't be blocked
 
## What needs to be tested:
- multiple DDA instances can be started that use the same sidecar

## What is possible in the future:
- timeouts in case connection to the sidecar is lost
- icnoming events to the DDA resource can be processed in parallel (especially
  important if workflows can have multiple functions being invoked at the same time)