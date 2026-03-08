"""
Sorting implementation. The agent can modify this file to improve throughput.

The function my_sort(arr) must return a sorted list in ascending order.
"""

import numpy as np


def my_sort(arr):
    """Use numpy's C-level introsort on a contiguous int array."""
    a = np.array(arr, dtype=np.int64)
    a.sort()
    return a.tolist()
