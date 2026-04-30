from __future__ import annotations

from typing import Any, Mapping, Optional, Union

import httpx

from ._common import build_auth_header, encode_path, filter_query, raise_for_response


class MarkdownFS:
    def __init__(
        self,
        base_url: str,
        *,
        token: Optional[str] = None,
        username: Optional[str] = None,
        timeout: float = 30.0,
        client: Optional[httpx.Client] = None,
    ) -> None:
        if not base_url:
            raise ValueError("base_url is required")
        self._base_url = base_url.rstrip("/")
        headers: dict[str, str] = {}
        auth = build_auth_header(token, username)
        if auth:
            headers["authorization"] = auth
        self._owns_client = client is None
        self._client = client or httpx.Client(
            base_url=self._base_url, headers=headers, timeout=timeout
        )
        if client is not None:
            self._client.headers.update(headers)

        self.fs = FsResource(self)
        self.search = SearchResource(self)
        self.vcs = VcsResource(self)

    def __enter__(self) -> "MarkdownFS":
        return self

    def __exit__(self, *exc: Any) -> None:
        self.close()

    def close(self) -> None:
        if self._owns_client:
            self._client.close()

    def _request(
        self,
        method: str,
        path: str,
        *,
        params: Optional[Mapping[str, Any]] = None,
        json: Any = None,
        content: Union[str, bytes, None] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> httpx.Response:
        response = self._client.request(
            method,
            path,
            params=filter_query(params),
            json=json,
            content=content,
            headers=dict(headers) if headers else None,
        )
        raise_for_response(response)
        return response

    def health(self) -> dict[str, Any]:
        return self._request("GET", "/health").json()

    def login(self, username: str) -> dict[str, Any]:
        return self._request("POST", "/auth/login", json={"username": username}).json()


class FsResource:
    def __init__(self, client: MarkdownFS) -> None:
        self._client = client

    def read(self, path: str) -> str:
        return self._client._request("GET", f"/fs/{encode_path(path)}").text

    def read_bytes(self, path: str) -> bytes:
        return self._client._request("GET", f"/fs/{encode_path(path)}").content

    def list(self, path: str = "") -> dict[str, Any]:
        url = f"/fs/{encode_path(path)}" if path else "/fs"
        return self._client._request("GET", url).json()

    def stat(self, path: str) -> dict[str, Any]:
        return self._client._request(
            "GET", f"/fs/{encode_path(path)}", params={"stat": True}
        ).json()

    def write(self, path: str, content: Union[str, bytes]) -> None:
        self._client._request(
            "PUT",
            f"/fs/{encode_path(path)}",
            content=content,
            headers={"content-type": "text/markdown"},
        )

    def mkdir(self, path: str) -> None:
        self._client._request(
            "PUT",
            f"/fs/{encode_path(path)}",
            headers={"x-markdownfs-type": "directory"},
        )

    def remove(self, path: str, *, recursive: bool = False) -> None:
        self._client._request(
            "DELETE", f"/fs/{encode_path(path)}", params={"recursive": recursive}
        )

    def copy(self, src: str, dst: str) -> None:
        self._client._request(
            "POST", f"/fs/{encode_path(src)}", params={"op": "copy", "dst": dst}
        )

    def move(self, src: str, dst: str) -> None:
        self._client._request(
            "POST", f"/fs/{encode_path(src)}", params={"op": "move", "dst": dst}
        )

    def tree(self, path: str = "") -> str:
        url = f"/tree/{encode_path(path)}" if path else "/tree"
        return self._client._request("GET", url).text


class SearchResource:
    def __init__(self, client: MarkdownFS) -> None:
        self._client = client

    def grep(
        self,
        pattern: str,
        *,
        path: Optional[str] = None,
        recursive: Optional[bool] = None,
    ) -> dict[str, Any]:
        return self._client._request(
            "GET",
            "/search/grep",
            params={"pattern": pattern, "path": path, "recursive": recursive},
        ).json()

    def find(
        self, *, path: Optional[str] = None, name: Optional[str] = None
    ) -> dict[str, Any]:
        return self._client._request(
            "GET", "/search/find", params={"path": path, "name": name}
        ).json()


class VcsResource:
    def __init__(self, client: MarkdownFS) -> None:
        self._client = client

    def commit(self, message: str) -> dict[str, Any]:
        return self._client._request(
            "POST", "/vcs/commit", json={"message": message}
        ).json()

    def log(self) -> dict[str, Any]:
        return self._client._request("GET", "/vcs/log").json()

    def revert(self, hash: str) -> None:
        self._client._request("POST", "/vcs/revert", json={"hash": hash})

    def status(self) -> str:
        return self._client._request("GET", "/vcs/status").text
