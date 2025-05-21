### Sqlx example

The example creates a workflow consisting of a chain of the following components:

- `sensor_simulator`: generates a random number in [0,20]
- `sqlx_test`: saves the value into a SQLite database, using a `database`
  resource

The state consists of the random value, which is initialized to a number
specified in the `sqlx_test` annotations, and a string that is equal to
`"initial"` upon initialization and `"subsequent"` for later updates.

First, build the `sqlx_test` WASM binary following the
[instructions](../../functions/README.md). 

Then you can start and stop the workflow with:

```
target/debug/edgeless_inabox
```

Then from another console:

```
ID=$(target/debug/edgeless_cli workflow start examples/sqlx/workflow.json)
```

To see the current content of the database:

```
echo "select * from WorkflowState" | sqlite3 sqlite.db
```

Example of output:

```
830aed5a-5873-4c80-9507-93db3a75b614|{"foo":16.5117,"bar":"subsequent"}
```

To stop the workflow:

```
target/debug/edgeless_cli workflow stop $ID
```
