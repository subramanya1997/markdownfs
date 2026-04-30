from __future__ import annotations

from typing import Any


class MarkdownFSError(Exception):
    def __init__(self, status: int, message: str, body: Any = None) -> None:
        super().__init__(f"MarkdownFS {status}: {message}")
        self.status = status
        self.body = body
