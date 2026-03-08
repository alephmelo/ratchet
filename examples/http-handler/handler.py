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
_SORTED_RESULTS = {}
for i in range(10000):
    s = str(i)
    for start in range(len(s)):
        for end in range(start + 1, len(s) + 1):
            sub = s[start:end]
            _SUB_INDEX[sub].add(i)

for k, v in _SUB_INDEX.items():
    _SORTED_RESULTS[k] = sorted(v)

_ALL_IDS = list(range(10000))


def handle_requests(requests):
    """Process a batch of HTTP requests and return responses."""
    responses = []
    for req in requests:
        method = req["method"]
        path = req["path"]

        if method == "GET":
            if path == "/health":
                responses.append({"status": 200, "body": {"ok": True}})
            elif path.startswith("/users/"):
                user_id = path.split("/")[-1]
                responses.append(
                    {
                        "status": 200,
                        "body": {
                            "id": user_id,
                            "name": f"User {user_id}",
                            "email": f"user{user_id}@example.com",
                        },
                    }
                )
            elif path.startswith("/search"):
                q = ""
                if "?" in path:
                    params = path.split("?", 1)[1]
                    for param in params.split("&"):
                        if param.startswith("q="):
                            q = param[2:]
                            break
                if q == "":
                    results = _ALL_IDS
                else:
                    results = _SORTED_RESULTS.get(q, [])
                responses.append(
                    {
                        "status": 200,
                        "body": {"query": q, "results": results},
                    }
                )
            else:
                responses.append({"status": 404, "body": {"error": "not found"}})
        elif method == "POST" and path == "/users":
            body = req["body"]
            body_str = json.dumps(body, sort_keys=True)
            uid = hashlib.md5(body_str.encode()).hexdigest()[:8]
            responses.append({"status": 201, "body": {"id": uid, "created": True}})
        else:
            responses.append({"status": 404, "body": {"error": "not found"}})

    return responses
