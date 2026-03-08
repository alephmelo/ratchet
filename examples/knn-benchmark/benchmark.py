"""
Benchmark for KNN classifier. DO NOT MODIFY.

Generates a synthetic classification dataset with known clusters,
runs predict() from knn.py, measures throughput and accuracy.
"""

import random
import time

from knn import predict

# --- Config ---
N_TRAIN = 2000  # training points
N_QUERY = 500  # query points
N_CLASSES = 10  # number of classes
DIMS = 16  # feature dimensions
K = 5  # neighbors
SEED = 42
N_WARMUP = 1  # warmup runs (not timed)
N_TIMED = 3  # timed runs


def generate_centroids(n_classes, dims, rng):
    """Generate random centroids for each class."""
    centroids = []
    for _ in range(n_classes):
        centroids.append([rng.gauss(0, 10) for _ in range(dims)])
    return centroids


def generate_data(n_points, centroids, dims, rng):
    """Generate clustered data around shared centroids."""
    n_classes = len(centroids)
    X = []
    y = []
    for i in range(n_points):
        label = i % n_classes
        centroid = centroids[label]
        point = [c + rng.gauss(0, 1.5) for c in centroid]
        X.append(point)
        y.append(label)

    return X, y


def check_accuracy(predicted, actual):
    """Compute classification accuracy."""
    correct = sum(1 for p, a in zip(predicted, actual) if p == a)
    return correct / len(actual) * 100.0


def main():
    rng = random.Random(SEED)

    # Generate deterministic dataset
    centroids = generate_centroids(N_CLASSES, DIMS, rng)
    train_X, train_y = generate_data(N_TRAIN, centroids, DIMS, rng)
    query_X, query_y = generate_data(N_QUERY, centroids, DIMS, rng)

    # Warmup
    for _ in range(N_WARMUP):
        predict(train_X, train_y, query_X, K)

    # Timed runs
    times = []
    last_predictions = None
    for _ in range(N_TIMED):
        start = time.perf_counter()
        predictions = predict(train_X, train_y, query_X, K)
        elapsed = time.perf_counter() - start
        times.append(elapsed)
        last_predictions = predictions

    avg_time = sum(times) / len(times)
    throughput = N_QUERY / avg_time  # queries per second

    accuracy = check_accuracy(last_predictions, query_y)

    print(f"throughput: {throughput:.2f}")
    print(f"accuracy: {accuracy:.1f}")
    print(f"elapsed_sec: {avg_time:.4f}")
    print(f"n_train: {N_TRAIN}")
    print(f"n_query: {N_QUERY}")
    print(f"dims: {DIMS}")
    print(f"k: {K}")


if __name__ == "__main__":
    main()
