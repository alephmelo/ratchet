"""
Sorting implementation. The agent can modify this file to improve throughput.

The function my_sort(arr) must return a sorted list in ascending order.
"""


def my_sort(arr):
    """In-place sort to avoid allocating a new list."""
    arr.sort()
    return arr
