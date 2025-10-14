### HTTP stress workflow

This workflow enables an HTTP endpoint where each request starts some computation on the nodes.

It consists of 2 functions:
1. Reads the request from the `http_ingress` resource and processes its body (if any)
2. Runs some computation on a node. Future work will enable the adjustement of the load.

If the body is not defined properly, the first function return code `422 Unprocessable Entity`
If the body is OK or there is no body, it will return code `200 OK`


The purpose of this workflow is to enable the integration of EDGELESS with other HTTP benchmark tools, that can provide custom traffic patterns.

### Example:

```
curl -H "Host: stress.edgeless-project.eu" -XPOST http://127.0.0.1:7008/hello
```
