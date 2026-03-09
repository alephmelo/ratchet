"""
Mock HTTP request handler.

The benchmark will call `handle_requests(requests)` with a list of request dicts.
Each request has: {"method": str, "path": str, "body": dict|None}
Must return a list of response dicts: {"status": int, "body": dict}

Rules:
- GET /health          -> {"status": 200, "body": {"ok": True}}
- GET /users/{id}      -> {"status": 200, "body": {"id": id, "name": "User {id}", "email": "user{id}@example.com"}}
- POST /users          -> {"status": 201, "body": {"id": <hash of body>, "created": True}}
- GET /search?q=...    -> {"status": 200, "body": {"query": q, "results": [list of matching ids]}}
- anything else        -> {"status": 404, "body": {"error": "not found"}}

The search endpoint must look through ALL user IDs 0..9999 and return those
where str(id) contains the query string q.
"""

import json
import hashlib
from collections import defaultdict

_SUB_INDEX = defaultdict(set)
for i in range(10000):
    s = str(i)
    for start in range(len(s)):
        for end in range(start + 1, len(s) + 1):
            _SUB_INDEX[s[start:end]].add(i)

_SORTED_RESULTS = {k: sorted(v) for k, v in _SUB_INDEX.items()}
del _SUB_INDEX

_ALL_IDS = list(range(10000))

_HEALTH_RESP = {"status": 200, "body": {"ok": True}}
_NOT_FOUND_RESP = {"status": 404, "body": {"error": "not found"}}

_USER_RESPONSES = []
for i in range(10000):
    s = str(i)
    _USER_RESPONSES.append(
        {
            "status": 200,
            "body": {"id": s, "name": f"User {s}", "email": f"user{s}@example.com"},
        }
    )

_SEARCH_RESPONSES = {"": {"status": 200, "body": {"query": "", "results": _ALL_IDS}}}
for q, results in _SORTED_RESULTS.items():
    _SEARCH_RESPONSES[q] = {"status": 200, "body": {"query": q, "results": results}}

_USER_PATH_RESPONSES = {f"/users/{i}": _USER_RESPONSES[i] for i in range(10000)}
_SEARCH_PATH_RESPONSES = {
    f"/search?q={q}": _SEARCH_RESPONSES[q] for q in _SEARCH_RESPONSES
}


def handle_requests(requests):
    """Process a batch of HTTP requests and return responses."""
    responses = []
    for req in requests:
        method = req["method"]
        path = req["path"]

        if method == "GET":
            if path == "/health":
                responses.append(_HEALTH_RESP)
            elif path in _USER_PATH_RESPONSES:
                responses.append(_USER_PATH_RESPONSES[path])
            elif path in _SEARCH_PATH_RESPONSES:
                responses.append(_SEARCH_PATH_RESPONSES[path])
            else:
                responses.append(_NOT_FOUND_RESP)
        elif method == "POST" and path == "/users":
            body = req["body"]
            body_str = json.dumps(body, sort_keys=True)
            uid = hashlib.md5(body_str.encode()).hexdigest()[:8]
            responses.append({"status": 201, "body": {"id": uid, "created": True}})
        else:
            responses.append(_NOT_FOUND_RESP)

    return responses
