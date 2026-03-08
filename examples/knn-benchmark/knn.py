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

from array import array
import heapq


def predict(train_X, train_y, query_X, k):
    """KNN using a KD-tree with bounding-box pruning and unrolled distance."""
    D = len(train_X[0])
    n_train = len(train_X)

    # Use array for cache-friendly flat storage
    flat = array("d")
    for p in train_X:
        flat.extend(p)

    labels = train_y

    # Build KD-tree with bounding boxes for pruning
    # Node format for internal: (dim, split_val, left, right, bbox_min, bbox_max)
    # Node format for leaf: list of (index, label) pairs + precomputed coords
    LEAF_SIZE = 32

    INF = float("inf")
    NINF = float("-inf")

    def build(indices, depth):
        n = len(indices)
        if n <= LEAF_SIZE:
            # Store leaf as tuple: (leaf_data,)
            # leaf_data is a tuple of (coord_tuple, label) for each point
            leaf_data = []
            for i in indices:
                base = i * D
                coords = (
                    flat[base],
                    flat[base + 1],
                    flat[base + 2],
                    flat[base + 3],
                    flat[base + 4],
                    flat[base + 5],
                    flat[base + 6],
                    flat[base + 7],
                    flat[base + 8],
                    flat[base + 9],
                    flat[base + 10],
                    flat[base + 11],
                    flat[base + 12],
                    flat[base + 13],
                    flat[base + 14],
                    flat[base + 15],
                )
                leaf_data.append((coords, labels[i]))
            return (leaf_data,)

        dim = depth % D
        indices.sort(key=lambda i: flat[i * D + dim])
        mid = n >> 1
        split_val = flat[indices[mid] * D + dim]
        left = build(indices[:mid], depth + 1)
        right = build(indices[mid:], depth + 1)
        return (dim, split_val, left, right)

    tree = build(list(range(n_train)), 0)

    # Query - optimized
    predictions = []
    heappush = heapq.heappush
    heapreplace = heapq.heapreplace
    pred_append = predictions.append

    for query in query_X:
        # Unpack query into local vars for fastest access
        q0, q1, q2, q3 = query[0], query[1], query[2], query[3]
        q4, q5, q6, q7 = query[4], query[5], query[6], query[7]
        q8, q9, q10, q11 = query[8], query[9], query[10], query[11]
        q12, q13, q14, q15 = query[12], query[13], query[14], query[15]
        q_tuple = (q0, q1, q2, q3, q4, q5, q6, q7, q8, q9, q10, q11, q12, q13, q14, q15)

        heap = []
        heap_size = 0
        worst = INF

        stack = [tree]
        while stack:
            node = stack.pop()
            node_len = len(node)

            if node_len == 1:
                # Leaf node - node[0] is list of (coords_tuple, label)
                leaf_data = node[0]
                for cl in leaf_data:
                    coords = cl[0]
                    # Unrolled distance computation for D=16 with early termination
                    diff = q0 - coords[0]
                    dist_sq = diff * diff
                    diff = q1 - coords[1]
                    dist_sq += diff * diff
                    diff = q2 - coords[2]
                    dist_sq += diff * diff
                    diff = q3 - coords[3]
                    dist_sq += diff * diff
                    if dist_sq >= worst:
                        continue
                    diff = q4 - coords[4]
                    dist_sq += diff * diff
                    diff = q5 - coords[5]
                    dist_sq += diff * diff
                    diff = q6 - coords[6]
                    dist_sq += diff * diff
                    diff = q7 - coords[7]
                    dist_sq += diff * diff
                    if dist_sq >= worst:
                        continue
                    diff = q8 - coords[8]
                    dist_sq += diff * diff
                    diff = q9 - coords[9]
                    dist_sq += diff * diff
                    diff = q10 - coords[10]
                    dist_sq += diff * diff
                    diff = q11 - coords[11]
                    dist_sq += diff * diff
                    if dist_sq >= worst:
                        continue
                    diff = q12 - coords[12]
                    dist_sq += diff * diff
                    diff = q13 - coords[13]
                    dist_sq += diff * diff
                    diff = q14 - coords[14]
                    dist_sq += diff * diff
                    diff = q15 - coords[15]
                    dist_sq += diff * diff
                    if dist_sq >= worst:
                        continue

                    neg = -dist_sq
                    if heap_size < k:
                        heappush(heap, (neg, cl[1]))
                        heap_size += 1
                        if heap_size == k:
                            worst = -heap[0][0]
                    else:
                        heapreplace(heap, (neg, cl[1]))
                        worst = -heap[0][0]
            else:
                dim, split_val, left, right = node
                diff = q_tuple[dim] - split_val
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
        pred_append(best_label)

    return predictions
