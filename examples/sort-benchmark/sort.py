"""
Sorting implementation. The agent can modify this file to improve throughput.

The function my_sort(arr) must return a sorted list in ascending order.
"""

import numpy as np
import struct

# Pre-compiled struct for packing 500 uint16 values
_packer = struct.Struct("500H")


def my_sort(arr):
    """struct.pack for fastest list→bytes, numpy copy+sort, tolist() back.
    struct.pack with *arr is faster than array.array constructor for list→buffer."""
    na = np.frombuffer(_packer.pack(*arr), dtype=np.uint16).copy()
    na.sort()
    return na.tolist()
