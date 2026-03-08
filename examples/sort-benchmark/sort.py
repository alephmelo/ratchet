"""
Sorting implementation. The agent can modify this file to improve throughput.

The function my_sort(arr) must return a sorted list in ascending order.
"""


def my_sort(arr):
    """Bubble sort — deliberately naive. Plenty of room for improvement."""
    n = len(arr)
    for i in range(n):
        for j in range(0, n - i - 1):
            if arr[j] > arr[j + 1]:
                arr[j], arr[j + 1] = arr[j + 1], arr[j]
    return arr
