### file-pusher example

The example creates a workflow triggered once per second where the content of
some files in a directory is sent by `file-pusher` to a `file-log` resource.

1. Build EDGELESS in debug mode, see [building instructions](../../BUILDING.md).

2. Create template configuration files, if not already created, with

```shell
target/debug/edgeless_inabox -t
target/debug/edgeless_cli -t cli.toml
```

3. Create a directory called `dataset` with some text files:

```shell
mkdir dataset
for (( i = 0 ; i < 5 ; i++ )) ; do echo -n $i > dataset/$i ; done
```

4. Configure the `file-pusher` resource provider by modifying `node.toml` as
   follows:

```ini
[resources.file_pusher_provider]
directory = "dataset/"
provider = "image-pusher-1"
```

5. Start EDGELESS in a box:

```shell
target/debug/edgeless_inabox
```

6. From another shell, start the example workflow in this directory:

```shell
ID=$(target/debug/edgeless_cli workflow start examples/file_pusher/workflow.json)
```

7. Check the content of the log file:

```shell
tail -f my-local-file.log
```

You should see this:

```
0
1
2
3
4
0
1
<...>
```