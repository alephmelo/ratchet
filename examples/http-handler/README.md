# http-handler

An AI agent optimizes a mock HTTP request handler for **throughput** and **tail latency** simultaneously, using ratchet's multi-metric Pareto optimization. This is the first example with two competing objectives.

Model used: `opencode/minimax-m2.5-free` (free via OpenCode).

## What's here

| File | Role |
|---|---|
| `handler.py` | **Editable** -- the agent's best result after optimization (precomputed lookup tables, ~1.85M req/s). |
| `benchmark.py` | **Immutable** -- generates 5000 mixed HTTP requests, times batch processing, measures throughput + p99 latency, validates correctness. |
| `ratchet.yaml` | Config -- **multi-metric Pareto**: maximize `requests_per_sec`, minimize `latency_p99`. `correctness` must stay at 100%. |
| `results.tsv` | Experiment log from a sample run. |

## Running

Generate the agent instructions:

```bash
ratchet init
```

Or let ratchet run the whole loop:

```bash
ratchet loop
```

Run the benchmark directly to see the current performance:

```bash
python3 benchmark.py
```

```
requests_per_sec: 5200.3
latency_p99: 0.290
correctness: 100.0
total_requests: 25000
elapsed_sec: 4.807
```

## Sample results

After running the optimization loop, view the scoreboard with `ratchet results`:

```
  http-handler — Pareto: requests_per_sec ^, latency_p99 v

  commit    requests_per_sec   latency_p99  vs base    status   description
  ------------------------------------------------------------------------------------------
  baseline         5200.00         0.290           + keep     baseline
  d5ee558       1141613.40          0.00  220x     ! crash    (correctness = 97.60)
  63ab6a4        707823.40          0.00  136x     + keep     precomputed substring index
  c72bf31       1267124.70          0.00  244x     + keep     precomputed user + search responses
  e3f77bc       1327175.20       0.00100  255x     + keep     optimized path dispatch
  8c1ab9e       1357241.80       0.00100  261x     + keep     cached response objects
  82a29c8       1852846.80       0.00100  356x     + keep     full path-keyed lookup table
  ------------------------------------------------------------------------------------------

  experiments: 20  (kept: 6, discarded: 8, crashed: 6)
  best requests_per_sec: 1852846.80  (356x vs baseline)  [82a29c8]
  best latency_p99: 0.00100  [e3f77bc]
  baseline:  requests_per_sec = 5200.00
  baseline:  latency_p99 = 0.290
```

The agent went from a naive per-request handler (5,200 req/s) to a fully precomputed lookup table (1,852,846 req/s) -- a **356x improvement** in 20 experiments. Tail latency dropped from 0.29ms to 0.001ms.

## Why this is a good test

- **Multi-metric tradeoff** -- throughput and tail latency can conflict. Precomputation helps both, but batching strategies may improve one at the cost of the other. This exercises ratchet's Pareto optimization.
- **Hard constraint** -- correctness must be exactly 100%. Six experiments crashed trying aggressive optimizations that broke response correctness.
- **Fast feedback** -- each run takes ~1 second.
- **Clear improvement gradient** -- the brute-force search over 10,000 IDs on every request is an obvious bottleneck. But going from "build an index" to "precompute everything" requires multiple creative leaps.

## Before and after

**Before** (baseline) -- naive handler, 5,200 req/s:

```python
def handle_requests(requests):
    responses = []
    for req in requests:
        method = req["method"]
        path = req["path"]
        if method == "GET" and path.startswith("/search"):
            q = ...  # parse query param
            results = []
            for i in range(10000):       # brute-force scan
                if q in str(i):
                    results.append(i)
            responses.append({"status": 200, "body": {"query": q, "results": results}})
        # ... other routes ...
    return responses
```

Every search request loops over 10,000 IDs and converts each to a string. Every user lookup constructs a fresh response dict. No caching, no precomputation.

**After** (optimized) -- precomputed lookup tables, 1,852,846 req/s:

```python
# At import time: build substring index, pre-create all response objects
_SORTED_RESULTS = {substring: sorted(matching_ids) for ...}
_USER_RESPONSES = [{"status": 200, "body": {...}} for i in range(10000)]
_USER_PATH_RESPONSES = {f"/users/{i}": _USER_RESPONSES[i] for i in range(10000)}
_SEARCH_PATH_RESPONSES = {f"/search?q={q}": resp for q, resp in ...}

def handle_requests(requests):
    responses = []
    for req in requests:
        if req["method"] == "GET":
            path = req["path"]
            if path in _USER_PATH_RESPONSES:       # O(1) dict lookup
                responses.append(_USER_PATH_RESPONSES[path])
            elif path in _SEARCH_PATH_RESPONSES:   # O(1) dict lookup
                responses.append(_SEARCH_PATH_RESPONSES[path])
            # ...
    return responses
```

The agent moved all computation to import time: substring index for search, pre-built response dicts for every possible user and search query, keyed by the full request path. At runtime, `handle_requests` is just a series of dict lookups.
