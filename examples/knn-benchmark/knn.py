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

import math


def predict(train_X, train_y, query_X, k):
    """Brute-force KNN — deliberately naive. Plenty of room for improvement."""
    predictions = []
    for query in query_X:
        # Compute distance to every training point
        distances = []
        for i, train_point in enumerate(train_X):
            dist = 0.0
            for d in range(len(query)):
                diff = query[d] - train_point[d]
                dist += diff * diff
            dist = math.sqrt(dist)
            distances.append((dist, train_y[i]))

        # Sort by distance, take k nearest
        distances.sort(key=lambda x: x[0])
        nearest = distances[:k]

        # Majority vote
        votes = {}
        for _, label in nearest:
            votes[label] = votes.get(label, 0) + 1
        best_label = max(votes, key=votes.get)
        predictions.append(best_label)

    return predictions
