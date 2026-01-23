# KPI 13 status

## Mandelbrot demo - local setup for demonstrations (visual setup)

This setup starts every component as a separate process in a tmux pane. The nodes can then be easily stopped by sending ctrl-c signal to them. The result of the failover can be seen in the browser.

### Prerequisites

1. Make sure that you've run `git submodule update --recursive` in the root of this repo.
2. install tmuxinator (e.g. `sudo apt install tmuxinator`)
3. `cd kpi13/`

### How to run

- Run: `scripts/build.sh` to build the binaries for edgeless
- Run: `tmuxinator stop kpi-13-demonstrator && tmuxinator start kpi-13-demonstrator`
- Run: `scripts/start_workflow.sh` to start the workflow (in a separate terminal window).

You may find the UI at `http://localhost:3000. Press the **Connect** button to connect to the demonstrator cluster.

>IMPORTANT: if you are running the demo on a remote machine, make sure to tunnel ports 3000 and 3002 to see the results in the local browser.

## KPI 13 evaluation

This describes how to generate results used for the final deliverable that describes the KPI13's work.

### Local experimenting (processes get killed on the host, no node resurrection)

```bash
cd kpi13/scripts
./host_experiment.sh
```

### How to evaluate results

1. Create Python Virtual Environment

```bash
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

2. Install Python Dependencies

```bash
pip install pandas plotnine numpy
```

3. Open the notebook and run the cells.