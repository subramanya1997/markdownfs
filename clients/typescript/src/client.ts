import { MarkdownFSError } from "./errors.js";
import type {
  ClientOptions,
  CommitResponse,
  FindOptions,
  FindResponse,
  GrepOptions,
  GrepResponse,
  HealthResponse,
  ListResponse,
  LoginResponse,
  LogResponse,
  RemoveOptions,
  StatResponse,
} from "./types.js";

const encodePath = (path: string): string =>
  path.replace(/^\/+/, "").split("/").map(encodeURIComponent).join("/");

export class MarkdownFS {
  readonly fs: FsResource;
  readonly search: SearchResource;
  readonly vcs: VcsResource;

  private readonly baseUrl: string;
  private readonly authHeader?: string;
  private readonly extraHeaders: Record<string, string>;
  private readonly fetchImpl: typeof fetch;

  constructor(opts: ClientOptions) {
    if (!opts.baseUrl) throw new Error("baseUrl is required");
    this.baseUrl = opts.baseUrl.replace(/\/+$/, "");
    this.fetchImpl = opts.fetch ?? globalThis.fetch.bind(globalThis);
    this.extraHeaders = opts.headers ?? {};
    if (opts.token) this.authHeader = `Bearer ${opts.token}`;
    else if (opts.username) this.authHeader = `User ${opts.username}`;

    this.fs = new FsResource(this);
    this.search = new SearchResource(this);
    this.vcs = new VcsResource(this);
  }

  async health(): Promise<HealthResponse> {
    return this.requestJson<HealthResponse>("GET", "/health");
  }

  async login(username: string): Promise<LoginResponse> {
    return this.requestJson<LoginResponse>("POST", "/auth/login", {
      json: { username },
    });
  }

  /** @internal */
  async requestJson<T>(
    method: string,
    path: string,
    init: { json?: unknown; query?: Record<string, string | boolean | undefined>; headers?: Record<string, string> } = {},
  ): Promise<T> {
    const res = await this.rawRequest(method, path, init);
    if (res.status === 204) return undefined as T;
    const text = await res.text();
    return text ? (JSON.parse(text) as T) : (undefined as T);
  }

  /** @internal */
  async rawRequest(
    method: string,
    path: string,
    init: {
      body?: BodyInit;
      json?: unknown;
      query?: Record<string, string | boolean | undefined>;
      headers?: Record<string, string>;
    } = {},
  ): Promise<Response> {
    const url = new URL(this.baseUrl + path);
    if (init.query) {
      for (const [k, v] of Object.entries(init.query)) {
        if (v !== undefined) url.searchParams.set(k, String(v));
      }
    }

    const headers: Record<string, string> = { ...this.extraHeaders, ...(init.headers ?? {}) };
    if (this.authHeader) headers.authorization = this.authHeader;

    let body = init.body;
    if (init.json !== undefined) {
      body = JSON.stringify(init.json);
      headers["content-type"] ??= "application/json";
    }

    const res = await this.fetchImpl(url.toString(), { method, headers, body });
    if (!res.ok) {
      const text = await res.text().catch(() => "");
      let parsed: unknown = text;
      let message = text || res.statusText;
      try {
        parsed = JSON.parse(text);
        if (parsed && typeof parsed === "object" && "error" in parsed) {
          message = String((parsed as { error: unknown }).error);
        }
      } catch {
        // body wasn't JSON; keep raw text
      }
      throw new MarkdownFSError(res.status, message, parsed);
    }
    return res;
  }
}

class FsResource {
  constructor(private readonly client: MarkdownFS) {}

  async read(path: string): Promise<string> {
    const res = await this.client.rawRequest("GET", `/fs/${encodePath(path)}`);
    return res.text();
  }

  async readBytes(path: string): Promise<Uint8Array> {
    const res = await this.client.rawRequest("GET", `/fs/${encodePath(path)}`);
    return new Uint8Array(await res.arrayBuffer());
  }

  async list(path: string = ""): Promise<ListResponse> {
    const url = path ? `/fs/${encodePath(path)}` : "/fs";
    return this.client.requestJson<ListResponse>("GET", url);
  }

  async stat(path: string): Promise<StatResponse> {
    return this.client.requestJson<StatResponse>("GET", `/fs/${encodePath(path)}`, {
      query: { stat: true },
    });
  }

  async write(path: string, content: string | Uint8Array): Promise<void> {
    await this.client.rawRequest("PUT", `/fs/${encodePath(path)}`, {
      body: content as BodyInit,
      headers: { "content-type": "text/markdown" },
    });
  }

  async mkdir(path: string): Promise<void> {
    await this.client.rawRequest("PUT", `/fs/${encodePath(path)}`, {
      headers: { "x-markdownfs-type": "directory" },
    });
  }

  async remove(path: string, opts: RemoveOptions = {}): Promise<void> {
    await this.client.rawRequest("DELETE", `/fs/${encodePath(path)}`, {
      query: { recursive: opts.recursive },
    });
  }

  async copy(src: string, dst: string): Promise<void> {
    await this.client.rawRequest("POST", `/fs/${encodePath(src)}`, {
      query: { op: "copy", dst },
    });
  }

  async move(src: string, dst: string): Promise<void> {
    await this.client.rawRequest("POST", `/fs/${encodePath(src)}`, {
      query: { op: "move", dst },
    });
  }

  async tree(path: string = ""): Promise<string> {
    const url = path ? `/tree/${encodePath(path)}` : "/tree";
    const res = await this.client.rawRequest("GET", url);
    return res.text();
  }
}

class SearchResource {
  constructor(private readonly client: MarkdownFS) {}

  async grep(pattern: string, opts: GrepOptions = {}): Promise<GrepResponse> {
    return this.client.requestJson<GrepResponse>("GET", "/search/grep", {
      query: { pattern, path: opts.path, recursive: opts.recursive },
    });
  }

  async find(opts: FindOptions = {}): Promise<FindResponse> {
    return this.client.requestJson<FindResponse>("GET", "/search/find", {
      query: { path: opts.path, name: opts.name },
    });
  }
}

class VcsResource {
  constructor(private readonly client: MarkdownFS) {}

  async commit(message: string): Promise<CommitResponse> {
    return this.client.requestJson<CommitResponse>("POST", "/vcs/commit", {
      json: { message },
    });
  }

  async log(): Promise<LogResponse> {
    return this.client.requestJson<LogResponse>("GET", "/vcs/log");
  }

  async revert(hash: string): Promise<void> {
    await this.client.requestJson<void>("POST", "/vcs/revert", { json: { hash } });
  }

  async status(): Promise<string> {
    const res = await this.client.rawRequest("GET", "/vcs/status");
    return res.text();
  }
}
