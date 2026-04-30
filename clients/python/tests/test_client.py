from __future__ import annotations

import httpx
import pytest
import respx

from markdownfs import AsyncMarkdownFS, MarkdownFS, MarkdownFSError


BASE = "http://x"


@respx.mock
def test_read_sends_bearer_token() -> None:
    route = respx.get(f"{BASE}/fs/notes/a.md").mock(
        return_value=httpx.Response(200, text="# hi")
    )
    with MarkdownFS(base_url=BASE, token="t0k") as mdfs:
        assert mdfs.fs.read("notes/a.md") == "# hi"
    assert route.calls.last.request.headers["authorization"] == "Bearer t0k"


@respx.mock
def test_write_encodes_path_and_sets_content_type() -> None:
    route = respx.put(f"{BASE}/fs/notes/has%20space.md").mock(
        return_value=httpx.Response(200, json={"written": "notes/has space.md", "size": 4})
    )
    with MarkdownFS(base_url=BASE) as mdfs:
        mdfs.fs.write("notes/has space.md", "body")
    req = route.calls.last.request
    assert req.headers["content-type"] == "text/markdown"
    assert req.content == b"body"


@respx.mock
def test_grep_passes_query_params() -> None:
    route = respx.get(f"{BASE}/search/grep").mock(
        return_value=httpx.Response(200, json={"results": [], "count": 0})
    )
    with MarkdownFS(base_url=BASE) as mdfs:
        mdfs.search.grep("TODO", recursive=True)
    url = route.calls.last.request.url
    assert url.params["pattern"] == "TODO"
    assert url.params["recursive"] == "true"


@respx.mock
def test_error_response_raises() -> None:
    respx.get(f"{BASE}/fs/missing.md").mock(
        return_value=httpx.Response(404, json={"error": "nope"})
    )
    with MarkdownFS(base_url=BASE) as mdfs:
        with pytest.raises(MarkdownFSError) as exc:
            mdfs.fs.read("missing.md")
    assert exc.value.status == 404


@respx.mock
def test_vcs_commit() -> None:
    respx.post(f"{BASE}/vcs/commit").mock(
        return_value=httpx.Response(200, json={"hash": "abc", "message": "snap", "author": "root"})
    )
    with MarkdownFS(base_url=BASE) as mdfs:
        result = mdfs.vcs.commit("snap")
    assert result["hash"] == "abc"


@respx.mock
async def test_async_client_read() -> None:
    respx.get(f"{BASE}/fs/a.md").mock(return_value=httpx.Response(200, text="# hi"))
    async with AsyncMarkdownFS(base_url=BASE) as mdfs:
        assert await mdfs.fs.read("a.md") == "# hi"


@respx.mock
async def test_async_client_commit() -> None:
    respx.post(f"{BASE}/vcs/commit").mock(
        return_value=httpx.Response(200, json={"hash": "z", "message": "m", "author": "root"})
    )
    async with AsyncMarkdownFS(base_url=BASE) as mdfs:
        result = await mdfs.vcs.commit("m")
    assert result["hash"] == "z"
