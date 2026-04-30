import { describe, expect, it } from "bun:test";
import { MarkdownFS } from "./client.js";
import { MarkdownFSError } from "./errors.js";

function mockFetch(handler: (req: Request) => Response | Promise<Response>) {
  return (async (input: RequestInfo | URL, init?: RequestInit) => {
    const req = new Request(input as RequestInfo, init);
    return handler(req);
  }) as unknown as typeof fetch;
}

describe("MarkdownFS", () => {
  it("sends bearer token and reads file body as text", async () => {
    const fetchImpl = mockFetch(async (req) => {
      expect(req.headers.get("authorization")).toBe("Bearer t0k");
      expect(new URL(req.url).pathname).toBe("/fs/notes/a.md");
      return new Response("# hi", { status: 200 });
    });
    const mdfs = new MarkdownFS({ baseUrl: "http://x", token: "t0k", fetch: fetchImpl });
    expect(await mdfs.fs.read("notes/a.md")).toBe("# hi");
  });

  it("encodes path segments and writes content", async () => {
    const fetchImpl = mockFetch(async (req) => {
      expect(req.method).toBe("PUT");
      expect(new URL(req.url).pathname).toBe("/fs/notes/has%20space.md");
      expect(await req.text()).toBe("body");
      return new Response(JSON.stringify({ written: "notes/has space.md", size: 4 }));
    });
    const mdfs = new MarkdownFS({ baseUrl: "http://x", fetch: fetchImpl });
    await mdfs.fs.write("notes/has space.md", "body");
  });

  it("passes query params for grep", async () => {
    const fetchImpl = mockFetch(async (req) => {
      const u = new URL(req.url);
      expect(u.pathname).toBe("/search/grep");
      expect(u.searchParams.get("pattern")).toBe("TODO");
      expect(u.searchParams.get("recursive")).toBe("true");
      return new Response(JSON.stringify({ results: [], count: 0 }));
    });
    const mdfs = new MarkdownFS({ baseUrl: "http://x", fetch: fetchImpl });
    await mdfs.search.grep("TODO", { recursive: true });
  });

  it("throws MarkdownFSError on non-ok response", async () => {
    const fetchImpl = mockFetch(async () =>
      new Response(JSON.stringify({ error: "nope" }), { status: 404 }),
    );
    const mdfs = new MarkdownFS({ baseUrl: "http://x", fetch: fetchImpl });
    await expect(mdfs.fs.read("missing.md")).rejects.toMatchObject({
      name: "MarkdownFSError",
      status: 404,
    });
    await expect(mdfs.fs.read("missing.md")).rejects.toBeInstanceOf(MarkdownFSError);
  });

  it("commits via vcs", async () => {
    const fetchImpl = mockFetch(async (req) => {
      expect(req.method).toBe("POST");
      expect(new URL(req.url).pathname).toBe("/vcs/commit");
      expect(await req.json()).toEqual({ message: "snap" });
      return new Response(JSON.stringify({ hash: "abc", message: "snap", author: "root" }));
    });
    const mdfs = new MarkdownFS({ baseUrl: "http://x", fetch: fetchImpl });
    const r = await mdfs.vcs.commit("snap");
    expect(r.hash).toBe("abc");
  });
});
