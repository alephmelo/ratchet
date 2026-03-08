"""
KNN classifier. The agent can modify this file to improve query throughput.

The function predict(train_X, train_y, query_X, k) must return a list of
predicted labels (integers) for each query point.

- train_X: list of N training points, each a list of D floats
- train_y: list of N integer labels
- query_X: list of Q query points, each a list of D floats
- k: number of neighbors

Returns: list of Q integer labels
"""

import heapq


def predict(train_X, train_y, query_X, k):
    """Optimized brute-force KNN."""
    # Pre-convert to tuples for faster iteration
    train_tuples = [tuple(p) for p in train_X]
    n_train = len(train_tuples)
    D = len(train_X[0])
    predictions = []

    for query in query_X:
        q = tuple(query)
        # Use a max-heap of size k to avoid full sort.
        # We store negative distances because heapq is a min-heap;
        # by negating, the largest distance is at the top and gets popped first.
        heap = []
        heap_size = 0

        for i in range(n_train):
            tp = train_tuples[i]
            # Inline squared distance (skip sqrt — monotonic)
            dist_sq = 0.0
            for d in range(D):
                diff = q[d] - tp[d]
                dist_sq += diff * diff
                # Early termination: if partial distance already exceeds
                # the worst in our heap, skip this point
                if heap_size == k and dist_sq >= -heap[0][0]:
                    break
            else:
                # Only reach here if loop completed without break
                if heap_size < k:
                    heapq.heappush(heap, (-dist_sq, train_y[i]))
                    heap_size += 1
                else:
                    # dist_sq < -heap[0][0] guaranteed by early termination check
                    heapq.heapreplace(heap, (-dist_sq, train_y[i]))

        # Majority vote
        votes = {}
        for _, label in heap:
            votes[label] = votes.get(label, 0) + 1
        best_label = max(votes, key=votes.get)
        predictions.append(best_label)

    return predictions
