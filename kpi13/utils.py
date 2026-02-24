"""Utility functions for KPI 13 evaluation notebook."""

import os

import pandas as pd
from plotnine import *  # type: ignore
from tqdm.auto import tqdm


# --- Data Loading ---

CSV_COLUMNS = [
    "timestamp_sec",
    "timestamp_nsec",
    "event_type",
    "span_id",
    "parent_span_id",
    "span_name",
    "level",
    "message",
]

SPAN_NAMES = ["kpi_13_failover"]


def load_results(results_dir: str = "./results/") -> pd.DataFrame:
    """Read all CSV files from *results_dir* and return a combined DataFrame."""
    csv_files = sorted(f for f in os.listdir(results_dir) if f.endswith(".csv"))
    print(f"Found {len(csv_files)} CSV files: {csv_files}")

    frames: list[pd.DataFrame] = []
    for csv_file in csv_files:
        filepath = os.path.join(results_dir, csv_file)
        try:
            df_temp = pd.read_csv(filepath, names=CSV_COLUMNS, skiprows=1)
            df_temp["source_file"] = csv_file
            frames.append(df_temp)
            print(f"  Loaded {len(df_temp)} rows from {csv_file}")
        except Exception as e:
            print(f"  Error reading {csv_file}: {e}")

    if not frames:
        raise RuntimeError("No CSV files could be loaded")

    df = pd.concat(frames, ignore_index=True)
    print(f"\nCombined dataset: {len(df)} total rows")
    return df

def compute_span_durations(df: pd.DataFrame, span_name: str) -> pd.DataFrame:
    """Match span_start / span_end events and compute durations.

    Only spans whose name matches *span_name* are included.
    """
    if span_name:
        # span_name is only set on span_start events; span_end has NaN.
        # Find the span_ids that match, then keep all events for those ids.
        matching_ids = df.loc[df["span_name"] == span_name, "span_id"].unique()
        filtered = df[df["span_id"].isin(matching_ids)]
    else:
        filtered = df
    records: list[dict] = []
    grouped = filtered.groupby("span_id")
    for span_id, group in tqdm(grouped, desc="Computing span durations", total=len(grouped)):
        if len(group) < 2:
            continue
        starts = group[group["event_type"] == "span_start"]
        ends = group[group["event_type"] == "span_end"]
        if starts.empty or ends.empty:
            continue

        start_row = starts.iloc[0]
        end_row = ends.iloc[-1]

        start_ns = int(start_row["timestamp_sec"]) * 1_000_000_000 + int(start_row["timestamp_nsec"])
        end_ns = int(end_row["timestamp_sec"]) * 1_000_000_000 + int(end_row["timestamp_nsec"])
        duration_ns = end_ns - start_ns

        records.append(
            {
                "span_name": start_row["span_name"],
                "start_time_ns": start_ns,
                "end_time_ns": end_ns,
                "duration_ns": duration_ns,
                "source_file": start_row["source_file"],
            }
        )

    return pd.DataFrame(records)


def summary_table(durations_df: pd.DataFrame) -> pd.DataFrame:
    """Return a per-run summary table with mean, std, p90, p99, p99.9, and max."""
    import numpy as np

    def _agg(g: pd.Series) -> pd.Series:
        return pd.Series(
            {
                "mean": g.mean(),
                "std": g.std(),
                "p90": np.percentile(g, 90),
                "p99": np.percentile(g, 99),
                "p99.9": np.percentile(g, 99.9),
                "max": g.max(),
            }
        )

    # Extract run number from filename for nicer ordering
    tmp = durations_df.copy()
    tmp["duration_ms"] = tmp["duration_ns"] / 1_000_000
    tmp["run"] = tmp["source_file"].str.extract(r"run(\d+)").astype(float)
    table = tmp.groupby("run")["duration_ms"].apply(_agg).unstack()
    table.index = table.index.astype(int)
    table.index.name = "exp. number"
    # Round for readability
    table = table.round(2)
    return table
