export interface ClientOptions {
  baseUrl: string;
  token?: string;
  username?: string;
  fetch?: typeof fetch;
  headers?: Record<string, string>;
}

export interface HealthResponse {
  status: string;
  version: string;
  commits: number;
  inodes: number;
  objects: number;
}

export interface LoginResponse {
  username: string;
  uid: number;
  gid: number;
  groups: number[];
}

export interface LsEntry {
  name: string;
  is_dir: boolean;
  is_symlink: boolean;
  size: number;
  mode: string;
  uid: number;
  gid: number;
  modified: number;
}

export interface ListResponse {
  entries: LsEntry[];
  path: string;
}

export interface StatResponse {
  inode_id: number;
  kind: string;
  size: number;
  mode: string;
  uid: number;
  gid: number;
  created: number;
  modified: number;
}

export interface GrepHit {
  file: string;
  line_num: number;
  line: string;
}

export interface GrepResponse {
  results: GrepHit[];
  count: number;
}

export interface FindResponse {
  results: string[];
  count: number;
}

export interface CommitResponse {
  hash: string;
  message: string;
  author: string;
}

export interface CommitEntry {
  hash: string;
  message: string;
  author: string;
  timestamp: number;
}

export interface LogResponse {
  commits: CommitEntry[];
}

export interface RemoveOptions {
  recursive?: boolean;
}

export interface GrepOptions {
  path?: string;
  recursive?: boolean;
}

export interface FindOptions {
  path?: string;
  name?: string;
}
