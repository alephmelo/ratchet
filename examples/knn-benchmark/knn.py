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
    """KNN using a KD-tree for fast neighbor lookup."""
    D = len(train_X[0])
    n_train = len(train_X)

    # Flatten training data for cache-friendly access
    flat = []
    for p in train_X:
        flat.extend(p)

    labels = train_y

    # Build KD-tree
    LEAF_SIZE = 24

    def build(indices, depth):
        n = len(indices)
        if n <= LEAF_SIZE:
            return indices  # leaf: just a list of indices

        dim = depth % D
        indices.sort(key=lambda i: flat[i * D + dim])
        mid = n >> 1
        split_val = flat[indices[mid] * D + dim]
        left = build(indices[:mid], depth + 1)
        right = build(indices[mid:], depth + 1)
        return (dim, split_val, left, right)

    tree = build(list(range(n_train)), 0)

    # Query
    predictions = []
    heappush = heapq.heappush
    heapreplace = heapq.heapreplace

    for query in query_X:
        q = query
        heap = []
        heap_size = 0
        worst = float("inf")

        stack = [tree]
        while stack:
            node = stack.pop()

            if isinstance(node, list):
                # Leaf node
                for i in node:
                    base = i * D
                    dist_sq = 0.0
                    d = 0
                    while d < D:
                        diff = q[d] - flat[base + d]
                        dist_sq += diff * diff
                        if dist_sq >= worst:
                            break
                        d += 1
                    else:
                        neg = -dist_sq
                        if heap_size < k:
                            heappush(heap, (neg, labels[i]))
                            heap_size += 1
                            if heap_size == k:
                                worst = -heap[0][0]
                        else:
                            heapreplace(heap, (neg, labels[i]))
                            worst = -heap[0][0]
            else:
                dim, split_val, left, right = node
                diff = q[dim] - split_val
                if diff <= 0:
                    near, far = left, right
                else:
                    near, far = right, left
                # Always explore near; only explore far if plane is close enough
                if diff * diff < worst:
                    stack.append(far)
                stack.append(near)

        # Majority vote
        votes = {}
        for _, label in heap:
            votes[label] = votes.get(label, 0) + 1
        best_label = max(votes, key=votes.get)
        predictions.append(best_label)

    return predictions
