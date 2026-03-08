# knn-benchmark

An AI agent optimizes a K-nearest neighbors classifier for throughput while maintaining accuracy. Pure Python — no external dependencies.

## What's here

| File | Role |
|---|---|
| `knn_baseline.py` | **Starting point** -- brute-force KNN with `math.sqrt` (~380 queries/sec). Copy to `knn.py` to start fresh. |
| `knn.py` | **Editable** -- the agent improves this. |
| `benchmark.py` | **Immutable** -- generates a synthetic 10-class dataset (2000 train, 500 query, 16 dims), runs `predict()`, measures throughput and accuracy. |
| `ratchet.yaml` | Config -- maximize `throughput`, `accuracy` must stay >= 90%. |

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

## Why this is a good test

- **Zero dependencies** -- pure Python stdlib only. Runs anywhere.
- **Clear improvement gradient** -- brute-force KNN is O(Q*N*D). Lots of well-known algorithmic speedups.
- **Hard constraint** -- accuracy must stay >= 90%, preventing degenerate shortcuts.
- **Room for creativity** -- the agent can try: skip sqrt, `heapq.nsmallest` instead of full sort, KD-tree, ball tree, `array` module, precomputed structures, C extensions via `ctypes`, etc.

## The baseline

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
