# knn-benchmark

An AI agent optimizes a K-nearest neighbors classifier for throughput while maintaining accuracy. Pure Python — no external dependencies.

## What's here

| File | Role |
|---|---|
| `knn_baseline.py` | **Starting point** -- brute-force KNN with `math.sqrt` (~380 queries/sec). Copy to `knn.py` to start fresh. |
| `knn.py` | **Editable** -- the agent's best result after optimization (KD-tree with variance-reordered dims, ~6,698 queries/sec). |
| `benchmark.py` | **Immutable** -- generates a synthetic 10-class dataset (2000 train, 500 query, 16 dims), runs `predict()`, measures throughput and accuracy. |
| `ratchet.yaml` | Config -- maximize `throughput`, `accuracy` must stay >= 90%. |
| `results.tsv` | Experiment log from a sample run. |

## Running

To start from scratch, copy the baseline over:

```bash
cp knn_baseline.py knn.py
```

Generate the agent instructions:

```bash
ratchet init
```

Or let ratchet run the whole loop:

```bash
ratchet loop --agent "opencode run < {prompt}"
```

Run the benchmark directly to see the baseline:

```bash
python3 benchmark.py
```

```
throughput: 382.78
accuracy: 100.0
elapsed_sec: 1.3062
n_train: 2000
n_query: 500
dims: 16
k: 5
```

## Sample results

After running the optimization loop, view the scoreboard with `ratchet results`:

```
  knn-benchmark — throughput ^ (higher is better)

  commit       throughput  accuracy  vs base    status   description
  ---------------------------------------------------------------------------------
  baseline        380.00    100.00            + keep     baseline
  8d7d584        1275.18    100.00  3x       + keep     heapq.nsmallest, skip sqrt, early termination on partial distance
  8979e8c        3277.71    100.00  9x       + keep     KD-tree with leaf_size=24, stack-based traversal
  1df30d6        4507.38    100.00  12x      + keep     array module for flat storage, unrolled 16-dim distance, bounding-box pruning
  ff6a482        5132.60    100.00  14x      + keep     inline top-5 tracking (no heapq), interleaved early termination every 2 dims
  a5f0885        6698.31    100.00  18x      + keep     variance-reordered dimensions for better pruning
  ---------------------------------------------------------------------------------

  experiments: 5  (kept: 5, discarded: 0, crashed: 0)
  best:        throughput = 6698.31  (18x vs baseline)  [a5f0885]
  baseline:    throughput = 380.00
```

The agent went from brute-force KNN (380 queries/sec) to a KD-tree with variance-reordered dimensions (6,698 queries/sec) -- an **18x improvement** in 5 experiments, with zero discards.

## Why this is a good test

- **Zero dependencies** -- pure Python stdlib only. Runs anywhere.
- **Clear improvement gradient** -- brute-force KNN is O(Q*N*D). Lots of well-known algorithmic speedups.
- **Hard constraint** -- accuracy must stay >= 90%, preventing degenerate shortcuts.
- **Room for creativity** -- the agent can try: skip sqrt, `heapq.nsmallest` instead of full sort, KD-tree, ball tree, `array` module, precomputed structures, C extensions via `ctypes`, etc.

## Before and after

**Before** (`knn_baseline.py`) -- brute-force KNN, 380 queries/sec:

```python
def predict(train_X, train_y, query_X, k):
    predictions = []
    for query in query_X:
        distances = []
        for i, train_point in enumerate(train_X):
            dist = 0.0
            for d in range(len(query)):
                diff = query[d] - train_point[d]
                dist += diff * diff
            dist = math.sqrt(dist)
            distances.append((dist, train_y[i]))
        distances.sort(key=lambda x: x[0])
        nearest = distances[:k]
        votes = {}
        for _, label in nearest:
            votes[label] = votes.get(label, 0) + 1
        best_label = max(votes, key=votes.get)
        predictions.append(best_label)
    return predictions
```

Three nested loops, `math.sqrt` on every distance, full sort just to get k smallest. Plenty to fix.

**After** (`knn.py`) -- KD-tree with variance-reordered dimensions, 6,698 queries/sec:

The agent built a KD-tree from scratch in pure Python using the `array` module for flat storage, with variance-ordered dimension splitting for better pruning, unrolled 16-dimension distance calculations, inline top-5 neighbor tracking (no heapq), and interleaved early termination. 436 lines of heavily optimized stdlib-only Python.
