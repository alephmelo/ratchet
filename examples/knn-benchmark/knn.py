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


def predict(train_X, train_y, query_X, k):
    """KNN using a flattened KD-tree with fixed-size top-k for k=5, D=16."""
    n_train = len(train_X)

    # Flat storage of all training points
    flat = array("d")
    for p in train_X:
        flat.extend(p)

    labels = train_y

    # Build KD-tree - leaf stores zipped (coord_tuple, label) pairs
    LEAF_SIZE = 20

    def build(indices, depth):
        n = len(indices)
        if n <= LEAF_SIZE:
            leaf_data = []
            la = leaf_data.append
            for i in indices:
                base = i << 4
                la(
                    (
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
                        labels[i],
                    )
                )
            return (tuple(leaf_data),)

        dim = depth & 15
        indices.sort(key=lambda i: flat[(i << 4) + dim])
        mid = n >> 1
        split_val = flat[(indices[mid] << 4) + dim]
        left = build(indices[:mid], depth + 1)
        right = build(indices[mid:], depth + 1)
        return (dim, split_val, left, right)

    tree = build(list(range(n_train)), 0)

    INF = float("inf")
    predictions = []
    pred_append = predictions.append

    for query in query_X:
        q0 = query[0]
        q1 = query[1]
        q2 = query[2]
        q3 = query[3]
        q4 = query[4]
        q5 = query[5]
        q6 = query[6]
        q7 = query[7]
        q8 = query[8]
        q9 = query[9]
        q10 = query[10]
        q11 = query[11]
        q12 = query[12]
        q13 = query[13]
        q14 = query[14]
        q15 = query[15]

        d0 = INF
        l0 = 0
        d1 = INF
        l1 = 0
        d2 = INF
        l2 = 0
        d3 = INF
        l3 = 0
        d4 = INF
        l4 = 0
        worst = INF

        qt = (q0, q1, q2, q3, q4, q5, q6, q7, q8, q9, q10, q11, q12, q13, q14, q15)

        stack = [tree]
        while stack:
            node = stack.pop()

            if len(node) == 1:
                # Leaf node - tuple of 17-element tuples (coords + label)
                for c in node[0]:
                    dd = q0 - c[0]
                    dist_sq = dd * dd
                    dd = q1 - c[1]
                    dist_sq += dd * dd
                    dd = q2 - c[2]
                    dist_sq += dd * dd
                    dd = q3 - c[3]
                    dist_sq += dd * dd
                    if dist_sq >= worst:
                        continue
                    dd = q4 - c[4]
                    dist_sq += dd * dd
                    dd = q5 - c[5]
                    dist_sq += dd * dd
                    dd = q6 - c[6]
                    dist_sq += dd * dd
                    dd = q7 - c[7]
                    dist_sq += dd * dd
                    if dist_sq >= worst:
                        continue
                    dd = q8 - c[8]
                    dist_sq += dd * dd
                    dd = q9 - c[9]
                    dist_sq += dd * dd
                    dd = q10 - c[10]
                    dist_sq += dd * dd
                    dd = q11 - c[11]
                    dist_sq += dd * dd
                    if dist_sq >= worst:
                        continue
                    dd = q12 - c[12]
                    dist_sq += dd * dd
                    dd = q13 - c[13]
                    dist_sq += dd * dd
                    dd = q14 - c[14]
                    dist_sq += dd * dd
                    dd = q15 - c[15]
                    dist_sq += dd * dd
                    if dist_sq >= worst:
                        continue

                    lbl = c[16]

                    # Insertion into sorted top-5 (d0 <= d1 <= d2 <= d3 <= d4)
                    if dist_sq < d0:
                        d4 = d3
                        l4 = l3
                        d3 = d2
                        l3 = l2
                        d2 = d1
                        l2 = l1
                        d1 = d0
                        l1 = l0
                        d0 = dist_sq
                        l0 = lbl
                    elif dist_sq < d1:
                        d4 = d3
                        l4 = l3
                        d3 = d2
                        l3 = l2
                        d2 = d1
                        l2 = l1
                        d1 = dist_sq
                        l1 = lbl
                    elif dist_sq < d2:
                        d4 = d3
                        l4 = l3
                        d3 = d2
                        l3 = l2
                        d2 = dist_sq
                        l2 = lbl
                    elif dist_sq < d3:
                        d4 = d3
                        l4 = l3
                        d3 = dist_sq
                        l3 = lbl
                    else:
                        d4 = dist_sq
                        l4 = lbl
                    worst = d4
            else:
                dim, split_val, left, right = node
                diff = qt[dim] - split_val
                if diff <= 0:
                    if diff * diff < worst:
                        stack.append(right)
                    stack.append(left)
                else:
                    if diff * diff < worst:
                        stack.append(left)
                    stack.append(right)

        # Majority vote for k=5 with 10 classes
        v = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        v[l0] += 1
        v[l1] += 1
        v[l2] += 1
        v[l3] += 1
        v[l4] += 1

        best_label = 0
        best_count = v[0]
        c = v[1]
        if c > best_count:
            best_count = c
            best_label = 1
        c = v[2]
        if c > best_count:
            best_count = c
            best_label = 2
        c = v[3]
        if c > best_count:
            best_count = c
            best_label = 3
        c = v[4]
        if c > best_count:
            best_count = c
            best_label = 4
        c = v[5]
        if c > best_count:
            best_count = c
            best_label = 5
        c = v[6]
        if c > best_count:
            best_count = c
            best_label = 6
        c = v[7]
        if c > best_count:
            best_count = c
            best_label = 7
        c = v[8]
        if c > best_count:
            best_count = c
            best_label = 8
        c = v[9]
        if c > best_count:
            best_count = c
            best_label = 9
        pred_append(best_label)

    return predictions
