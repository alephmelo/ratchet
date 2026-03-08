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
    """KNN using KD-tree with variance-reordered dims, initial seeding, and aggressive pruning."""
    n_train = len(train_X)
    D = 16

    # Compute variance per dimension to reorder for better early termination
    means = [0.0] * D
    for p in train_X:
        for d in range(D):
            means[d] += p[d]
    inv_n = 1.0 / n_train
    for d in range(D):
        means[d] *= inv_n

    variances = [0.0] * D
    for p in train_X:
        for d in range(D):
            diff = p[d] - means[d]
            variances[d] += diff * diff

    dim_order = sorted(range(D), key=lambda d: variances[d], reverse=True)
    do0, do1, do2, do3, do4, do5, do6, do7 = (
        dim_order[0],
        dim_order[1],
        dim_order[2],
        dim_order[3],
        dim_order[4],
        dim_order[5],
        dim_order[6],
        dim_order[7],
    )
    do8, do9, do10, do11, do12, do13, do14, do15 = (
        dim_order[8],
        dim_order[9],
        dim_order[10],
        dim_order[11],
        dim_order[12],
        dim_order[13],
        dim_order[14],
        dim_order[15],
    )

    # Flat storage with reordered dimensions
    flat = array("d")
    for p in train_X:
        flat.append(p[do0])
        flat.append(p[do1])
        flat.append(p[do2])
        flat.append(p[do3])
        flat.append(p[do4])
        flat.append(p[do5])
        flat.append(p[do6])
        flat.append(p[do7])
        flat.append(p[do8])
        flat.append(p[do9])
        flat.append(p[do10])
        flat.append(p[do11])
        flat.append(p[do12])
        flat.append(p[do13])
        flat.append(p[do14])
        flat.append(p[do15])

    labels = train_y

    LEAF_SIZE = 24

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
                        flat[base | 1],
                        flat[base | 2],
                        flat[base | 3],
                        flat[base | 4],
                        flat[base | 5],
                        flat[base | 6],
                        flat[base | 7],
                        flat[base | 8],
                        flat[base | 9],
                        flat[base | 10],
                        flat[base | 11],
                        flat[base | 12],
                        flat[base | 13],
                        flat[base | 14],
                        flat[base | 15],
                        labels[i],
                    )
                )
            return (tuple(leaf_data),)

        dim = depth & 15
        indices.sort(key=lambda i: flat[(i << 4) | dim])
        mid = n >> 1
        split_val = flat[(indices[mid] << 4) | dim]
        left = build(indices[:mid], depth + 1)
        right = build(indices[mid:], depth + 1)
        return (dim, split_val, left, right)

    tree = build(list(range(n_train)), 0)

    # Precompute reordered queries as tuples
    rq = []
    for q in query_X:
        rq.append(
            (
                q[do0],
                q[do1],
                q[do2],
                q[do3],
                q[do4],
                q[do5],
                q[do6],
                q[do7],
                q[do8],
                q[do9],
                q[do10],
                q[do11],
                q[do12],
                q[do13],
                q[do14],
                q[do15],
            )
        )

    INF = float("inf")
    predictions = []
    pred_append = predictions.append

    for query in rq:
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

        # Phase 1: Walk down to closest leaf and collect siblings
        node = tree
        siblings = []
        while len(node) != 1:
            dim, split_val, left, right = node
            diff = query[dim] - split_val
            if diff <= 0:
                siblings.append((right, diff * diff))
                node = left
            else:
                siblings.append((left, diff * diff))
                node = right

        # Seed from closest leaf
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

        for c in node[0]:
            dd = q0 - c[0]
            dist_sq = dd * dd
            dd = q1 - c[1]
            dist_sq += dd * dd
            if dist_sq >= worst:
                continue
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
            if dist_sq >= worst:
                continue
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
            if dist_sq >= worst:
                continue
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
            if dist_sq >= worst:
                continue
            dd = q14 - c[14]
            dist_sq += dd * dd
            dd = q15 - c[15]
            dist_sq += dd * dd
            if dist_sq >= worst:
                continue
            lbl = c[16]
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

        # Phase 2: Check sibling subtrees
        stack = []
        for sib, sdist in siblings:
            if sdist < worst:
                stack.append(sib)

        while stack:
            node = stack.pop()
            if len(node) == 1:
                for c in node[0]:
                    dd = q0 - c[0]
                    dist_sq = dd * dd
                    dd = q1 - c[1]
                    dist_sq += dd * dd
                    if dist_sq >= worst:
                        continue
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
                    if dist_sq >= worst:
                        continue
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
                    if dist_sq >= worst:
                        continue
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
                    if dist_sq >= worst:
                        continue
                    dd = q14 - c[14]
                    dist_sq += dd * dd
                    dd = q15 - c[15]
                    dist_sq += dd * dd
                    if dist_sq >= worst:
                        continue
                    lbl = c[16]
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
                diff = query[dim] - split_val
                if diff <= 0:
                    if diff * diff < worst:
                        stack.append(right)
                    stack.append(left)
                else:
                    if diff * diff < worst:
                        stack.append(left)
                    stack.append(right)

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
