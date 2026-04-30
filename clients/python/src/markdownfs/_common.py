from __future__ import annotations

from typing import Any, Mapping, Optional
from urllib.parse import quote

import httpx

from .errors import MarkdownFSError


def encode_path(path: str) -> str:
    parts = path.lstrip("/").split("/")
    return "/".join(quote(p, safe="") for p in parts)


def build_auth_header(token: Optional[str], username: Optional[str]) -> Optional[str]:
    if token:
        return f"Bearer {token}"
    if username:
        return f"User {username}"
    return None


def filter_query(params: Optional[Mapping[str, Any]]) -> dict[str, Any]:
    if not params:
        return {}
    out: dict[str, Any] = {}
    for k, v in params.items():
        if v is None:
            continue
        if isinstance(v, bool):
            out[k] = "true" if v else "false"
        else:
            out[k] = v
    return out


def raise_for_response(response: httpx.Response) -> None:
    if response.is_success:
        return
    text = response.text
    body: Any = text
    message = text or response.reason_phrase
    try:
        body = response.json()
        if isinstance(body, dict) and "error" in body:
            message = str(body["error"])
    except Exception:
        pass
    raise MarkdownFSError(response.status_code, message, body)
