from __future__ import annotations

from typing import Any, Mapping, Optional, Union

import httpx

from ._common import build_auth_header, encode_path, filter_query, raise_for_response


class AsyncMarkdownFS:
    def __init__(
        self,
        base_url: str,
        *,
        token: Optional[str] = None,
        username: Optional[str] = None,
        timeout: float = 30.0,
        client: Optional[httpx.AsyncClient] = None,
    ) -> None:
        if not base_url:
            raise ValueError("base_url is required")
        self._base_url = base_url.rstrip("/")
        headers: dict[str, str] = {}
        auth = build_auth_header(token, username)
        if auth:
            headers["authorization"] = auth
        self._owns_client = client is None
        self._client = client or httpx.AsyncClient(
            base_url=self._base_url, headers=headers, timeout=timeout
        )
        if client is not None:
            self._client.headers.update(headers)

        self.fs = AsyncFsResource(self)
        self.search = AsyncSearchResource(self)
        self.vcs = AsyncVcsResource(self)

    async def __aenter__(self) -> "AsyncMarkdownFS":
        return self

    async def __aexit__(self, *exc: Any) -> None:
        await self.aclose()

    async def aclose(self) -> None:
        if self._owns_client:
            await self._client.aclose()

    async def _request(
        self,
        method: str,
        path: str,
        *,
        params: Optional[Mapping[str, Any]] = None,
        json: Any = None,
        content: Union[str, bytes, None] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> httpx.Response:
        response = await self._client.request(
            method,
            path,
            params=filter_query(params),
            json=json,
            content=content,
            headers=dict(headers) if headers else None,
        )
        raise_for_response(response)
        return response

    async def health(self) -> dict[str, Any]:
        return (await self._request("GET", "/health")).json()

    async def login(self, username: str) -> dict[str, Any]:
        return (
            await self._request("POST", "/auth/login", json={"username": username})
        ).json()


class AsyncFsResource:
    def __init__(self, client: AsyncMarkdownFS) -> None:
        self._client = client

    async def read(self, path: str) -> str:
        return (await self._client._request("GET", f"/fs/{encode_path(path)}")).text

    async def read_bytes(self, path: str) -> bytes:
        return (await self._client._request("GET", f"/fs/{encode_path(path)}")).content

    async def list(self, path: str = "") -> dict[str, Any]:
        url = f"/fs/{encode_path(path)}" if path else "/fs"
        return (await self._client._request("GET", url)).json()

    async def stat(self, path: str) -> dict[str, Any]:
        return (
            await self._client._request(
                "GET", f"/fs/{encode_path(path)}", params={"stat": True}
            )
        ).json()

    async def write(self, path: str, content: Union[str, bytes]) -> None:
        await self._client._request(
            "PUT",
            f"/fs/{encode_path(path)}",
            content=content,
            headers={"content-type": "text/markdown"},
        )

    async def mkdir(self, path: str) -> None:
        await self._client._request(
            "PUT",
            f"/fs/{encode_path(path)}",
            headers={"x-markdownfs-type": "directory"},
        )

    async def remove(self, path: str, *, recursive: bool = False) -> None:
        await self._client._request(
            "DELETE", f"/fs/{encode_path(path)}", params={"recursive": recursive}
        )

    async def copy(self, src: str, dst: str) -> None:
        await self._client._request(
            "POST", f"/fs/{encode_path(src)}", params={"op": "copy", "dst": dst}
        )

    async def move(self, src: str, dst: str) -> None:
        await self._client._request(
            "POST", f"/fs/{encode_path(src)}", params={"op": "move", "dst": dst}
        )

    async def tree(self, path: str = "") -> str:
        url = f"/tree/{encode_path(path)}" if path else "/tree"
        return (await self._client._request("GET", url)).text


class AsyncSearchResource:
    def __init__(self, client: AsyncMarkdownFS) -> None:
        self._client = client

    async def grep(
        self,
        pattern: str,
        *,
        path: Optional[str] = None,
        recursive: Optional[bool] = None,
    ) -> dict[str, Any]:
        return (
            await self._client._request(
                "GET",
                "/search/grep",
                params={"pattern": pattern, "path": path, "recursive": recursive},
            )
        ).json()

    async def find(
        self, *, path: Optional[str] = None, name: Optional[str] = None
    ) -> dict[str, Any]:
        return (
            await self._client._request(
                "GET", "/search/find", params={"path": path, "name": name}
            )
        ).json()


class AsyncVcsResource:
    def __init__(self, client: AsyncMarkdownFS) -> None:
        self._client = client

    async def commit(self, message: str) -> dict[str, Any]:
        return (
            await self._client._request(
                "POST", "/vcs/commit", json={"message": message}
            )
        ).json()

    async def log(self) -> dict[str, Any]:
        return (await self._client._request("GET", "/vcs/log")).json()

    async def revert(self, hash: str) -> None:
        await self._client._request("POST", "/vcs/revert", json={"hash": hash})

    async def status(self) -> str:
        return (await self._client._request("GET", "/vcs/status")).text
