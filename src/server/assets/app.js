// MarkdownFS minimal web UI. Pure REST client.

const $ = (id) => document.getElementById(id);
const TOKEN_KEY = "mdfs.token";
const USER_KEY = "mdfs.username";
const OBO_KEY = "mdfs.onBehalfOf";

const state = {
  expanded: new Set([""]),
  selected: null,
  editing: false,
  originalContent: "",
  token: localStorage.getItem(TOKEN_KEY) || null,
  onBehalfOf: localStorage.getItem(OBO_KEY) || null,
  user: null,
  isAdmin: false,
};

function setToken(token, username) {
  if (token) {
    localStorage.setItem(TOKEN_KEY, token);
    if (username) localStorage.setItem(USER_KEY, username);
  } else {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(USER_KEY);
  }
  state.token = token;
}

function setOnBehalfOf(value) {
  if (value) {
    localStorage.setItem(OBO_KEY, value);
    state.onBehalfOf = value;
  } else {
    localStorage.removeItem(OBO_KEY);
    state.onBehalfOf = null;
  }
}

async function api(method, path, opts = {}) {
  const init = { method, headers: {} };
  if (opts.json !== undefined) {
    init.body = JSON.stringify(opts.json);
    init.headers["content-type"] = "application/json";
  } else if (opts.body !== undefined) {
    init.body = opts.body;
    init.headers["content-type"] = opts.contentType || "text/markdown";
  }
  if (!opts.skipAuth) {
    if (state.token) {
      init.headers["authorization"] = `Bearer ${state.token}`;
    } else if (state.usernameAuth) {
      init.headers["authorization"] = `User ${state.usernameAuth}`;
    } else if (opts.username) {
      init.headers["authorization"] = `User ${opts.username}`;
    }
    if (state.onBehalfOf) {
      init.headers["x-markdownfs-on-behalf-of"] = state.onBehalfOf;
    }
  }
  if (opts.headers) Object.assign(init.headers, opts.headers);

  const res = await fetch(path, init);
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    let msg = res.statusText;
    try {
      const j = JSON.parse(text);
      if (j && j.error) msg = j.error;
    } catch {}
    const err = new Error(msg);
    err.status = res.status;
    throw err;
  }
  return res;
}

async function copyToClipboard(text, btn) {
  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
    } else {
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.append(ta);
      ta.select();
      document.execCommand("copy");
      ta.remove();
    }
    if (btn) {
      const original = btn.textContent;
      btn.textContent = "copied ✓";
      btn.classList.add("copied");
      setTimeout(() => {
        btn.textContent = original;
        btn.classList.remove("copied");
      }, 1500);
    }
    return true;
  } catch (e) {
    toast(`copy failed: ${e.message}`, true);
    return false;
  }
}

function toast(msg, isError = false) {
  const el = $("toast");
  el.textContent = msg;
  el.classList.toggle("error", isError);
  el.hidden = false;
  clearTimeout(toast._t);
  toast._t = setTimeout(() => (el.hidden = true), 2200);
}

function ts(seconds) {
  const d = new Date(seconds * 1000);
  const diff = (Date.now() - d.getTime()) / 1000;
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  return d.toISOString().slice(0, 10);
}

async function refreshHeader() {
  try {
    const res = await api("GET", "/health");
    const h = await res.json();
    $("counters").textContent = `· ${h.inodes} inodes · ${h.commits} commits`;
    return h;
  } catch (e) {
    $("counters").textContent = "(server unreachable)";
    return null;
  }
}

async function refreshWhoAmI() {
  try {
    const res = await api("GET", "/auth/whoami");
    const w = await res.json();
    state.user = w;
    state.isAdmin = w.is_root || w.is_wheel === true;
    $("user-info").hidden = false;
    let label = w.authenticated ? w.username : `${w.username} (anon)`;
    if (w.on_behalf_of) {
      label += ` → ${w.on_behalf_of}`;
    }
    $("user-name").textContent = label;
    $("users-btn").hidden = !state.isAdmin;
    return w;
  } catch (e) {
    state.user = null;
    state.isAdmin = false;
    $("user-info").hidden = true;
    return null;
  }
}

function showBackdrop(which) {
  $("modal-backdrop").hidden = false;
  $("bootstrap-modal").hidden = which !== "bootstrap";
  $("login-modal").hidden = which !== "login";
}

function hideBackdrop() {
  $("modal-backdrop").hidden = true;
}

async function tryBootstrap(username) {
  const res = await api("POST", "/auth/bootstrap", {
    json: { username },
    skipAuth: true,
  });
  return await res.json();
}

async function tryLoginUsername(username) {
  await api("POST", "/auth/login", { json: { username }, skipAuth: true });
  // No token from username login — store username and use User auth header.
  // For simplicity, we set state.token to null and rely on opts.username,
  // but localStorage tracks the username so subsequent loads keep us signed in.
  setToken(null, username);
  state.usernameAuth = username;
}

function logout() {
  setToken(null);
  setOnBehalfOf(null);
  state.usernameAuth = null;
  location.reload();
}

async function loadDir(path) {
  const url = path ? `/fs/${encodeSegments(path)}` : "/fs";
  const res = await api("GET", url);
  const data = await res.json();
  return data.entries || [];
}

function encodeSegments(p) {
  return p.replace(/^(\.?\/)+|\/+$/g, "").split("/").map(encodeURIComponent).join("/");
}

async function renderTree() {
  const root = $("tree");
  root.innerHTML = "";
  await renderDir(root, "");
}

async function renderDir(parent, path) {
  let entries;
  try {
    entries = await loadDir(path);
  } catch (e) {
    return;
  }
  entries.sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  for (const e of entries) {
    const childPath = path ? `${path}/${e.name}` : e.name;
    const li = document.createElement("li");
    const item = document.createElement("div");
    item.className = "tree-item";
    item.style.paddingLeft = `${path.split("/").filter(Boolean).length * 14 + 8}px`;

    const twirl = document.createElement("span");
    twirl.className = "twirl";
    twirl.textContent = e.is_dir ? (state.expanded.has(childPath) ? "▼" : "▶") : "";

    const icon = document.createElement("span");
    icon.className = "icon";
    icon.textContent = e.is_dir ? "📁" : "📄";

    const name = document.createElement("span");
    name.className = "name";
    name.textContent = e.name;

    item.append(twirl, icon, name);
    li.append(item);

    if (e.is_dir) {
      const childUl = document.createElement("ul");
      if (state.expanded.has(childPath)) {
        await renderDir(childUl, childPath);
      } else {
        childUl.hidden = true;
      }
      li.append(childUl);

      item.addEventListener("click", async () => {
        if (state.expanded.has(childPath)) {
          state.expanded.delete(childPath);
          twirl.textContent = "▶";
          childUl.hidden = true;
        } else {
          state.expanded.add(childPath);
          twirl.textContent = "▼";
          if (!childUl.children.length) {
            await renderDir(childUl, childPath);
          }
          childUl.hidden = false;
        }
      });
    } else {
      item.addEventListener("click", () => openFile(childPath, item));
    }
    parent.append(li);
  }
}

async function openFile(path, itemEl) {
  if (state.editing && !confirm("Discard unsaved changes?")) return;

  document.querySelectorAll(".tree-item.selected").forEach((el) => el.classList.remove("selected"));
  if (itemEl) itemEl.classList.add("selected");

  try {
    const res = await api("GET", `/fs/${encodeSegments(path)}`);
    const text = await res.text();
    state.selected = path;
    state.originalContent = text;
    state.editing = false;
    $("placeholder").hidden = true;
    $("content").hidden = false;
    $("content").textContent = text;
    $("editor").hidden = true;
    $("current-path").textContent = path;
    $("file-actions").hidden = false;
    $("edit").hidden = false;
    $("save").hidden = true;
    $("cancel").hidden = true;
    // Fetch metadata and show mode/owner
    try {
      const stRes = await api("GET", `/fs/${encodeSegments(path)}?stat=true`);
      const st = await stRes.json();
      state.selectedMode = st.mode;
      const owner = await uidToName(st.uid);
      state.selectedOwner = owner;
      $("file-meta").textContent = `${st.mode} ${owner}`;
    } catch {
      $("file-meta").textContent = "";
    }
  } catch (e) {
    toast(`open: ${e.message}`, true);
  }
}

let userNameCache = null;
async function uidToName(uid) {
  if (uid === 0) return "root";
  if (!state.isAdmin) return `uid=${uid}`;
  if (!userNameCache) {
    try {
      const res = await api("GET", "/admin/users");
      const data = await res.json();
      userNameCache = new Map((data.users || []).map((u) => [u.uid, u.name]));
    } catch {
      return `uid=${uid}`;
    }
  }
  return userNameCache.get(uid) || `uid=${uid}`;
}

function startEdit() {
  if (!state.selected) return;
  state.editing = true;
  const ed = $("editor");
  ed.value = state.originalContent;
  ed.hidden = false;
  $("content").hidden = true;
  $("edit").hidden = true;
  $("save").hidden = false;
  $("cancel").hidden = false;
  ed.focus();
}

function cancelEdit() {
  state.editing = false;
  $("editor").hidden = true;
  $("content").hidden = false;
  $("edit").hidden = false;
  $("save").hidden = true;
  $("cancel").hidden = true;
}

async function saveEdit() {
  if (!state.selected) return;
  const text = $("editor").value;
  try {
    await api("PUT", `/fs/${encodeSegments(state.selected)}`, { body: text });
    state.originalContent = text;
    $("content").textContent = text;
    cancelEdit();
    toast("saved");
    await refreshHeader();
  } catch (e) {
    toast(`save: ${e.message}`, true);
  }
}

async function deleteFile() {
  if (!state.selected) return;
  if (!confirm(`Delete ${state.selected}?`)) return;
  try {
    await api("DELETE", `/fs/${encodeSegments(state.selected)}`);
    state.selected = null;
    state.editing = false;
    $("content").hidden = true;
    $("editor").hidden = true;
    $("placeholder").hidden = false;
    $("current-path").textContent = "no file selected";
    $("file-actions").hidden = true;
    toast("deleted");
    await renderTree();
    await refreshHeader();
  } catch (e) {
    toast(`delete: ${e.message}`, true);
  }
}

async function newFile() {
  const path = prompt("New file path (e.g. notes/idea.md):");
  if (!path) return;
  try {
    await api("PUT", `/fs/${encodeSegments(path)}`, { body: "" });
    toast(`created ${path}`);
    const segs = path.split("/").slice(0, -1);
    let acc = "";
    for (const s of segs) {
      acc = acc ? `${acc}/${s}` : s;
      state.expanded.add(acc);
    }
    await renderTree();
    await refreshHeader();
    await openFile(path, null);
  } catch (e) {
    toast(`new: ${e.message}`, true);
  }
}

async function commitWorking() {
  const message = prompt("Commit message:");
  if (!message) return;
  try {
    const res = await api("POST", "/vcs/commit", { json: { message } });
    const data = await res.json();
    toast(`committed ${data.hash}`);
    await refreshHeader();
    await refreshCommits();
  } catch (e) {
    toast(`commit: ${e.message}`, true);
  }
}

async function refreshCommits() {
  try {
    const res = await api("GET", "/vcs/log");
    const data = await res.json();
    const ul = $("commits");
    ul.innerHTML = "";
    for (const c of data.commits || []) {
      const li = document.createElement("li");
      li.innerHTML = `<span class="hash">${c.hash}</span><span class="msg"></span><span class="author"></span><span class="when"></span>`;
      li.querySelector(".msg").textContent = c.message;
      li.querySelector(".author").textContent = c.author;
      li.querySelector(".when").textContent = ts(c.timestamp);
      ul.append(li);
    }
    $("commit-count").textContent = `(${(data.commits || []).length})`;
  } catch (e) {
    // ignore
  }
}

let searchTimer;
function onSearchInput(e) {
  const pattern = e.target.value.trim();
  clearTimeout(searchTimer);
  if (!pattern) {
    $("search-results").hidden = true;
    return;
  }
  searchTimer = setTimeout(async () => {
    try {
      const url = `/search/grep?pattern=${encodeURIComponent(pattern)}&recursive=true`;
      const res = await api("GET", url);
      const data = await res.json();
      const ul = $("hits");
      ul.innerHTML = "";
      for (const r of data.results || []) {
        const li = document.createElement("li");
        li.innerHTML = `<span class="file"></span><span class="line"></span>`;
        li.querySelector(".file").textContent = `${r.file}:${r.line_num}`;
        li.querySelector(".line").textContent = r.line.length > 100 ? r.line.slice(0, 100) + "…" : r.line;
        li.addEventListener("click", () => openFile(r.file.replace(/^(\.?\/)+/, ""), null));
        ul.append(li);
      }
      $("search-results").hidden = false;
    } catch (e) {
      toast(`search: ${e.message}`, true);
    }
  }, 250);
}

function bind() {
  $("edit").addEventListener("click", startEdit);
  $("save").addEventListener("click", saveEdit);
  $("cancel").addEventListener("click", cancelEdit);
  $("delete").addEventListener("click", deleteFile);
  $("new-file").addEventListener("click", newFile);
  $("commit").addEventListener("click", commitWorking);
  $("search").addEventListener("input", onSearchInput);
  $("commits-toggle").addEventListener("click", () => {
    $("commits-panel").classList.toggle("collapsed");
  });
  $("logout").addEventListener("click", logout);
  $("users-btn").addEventListener("click", openUsersPanel);
  $("users-close").addEventListener("click", closeUsersPanel);
  $("chmod-btn").addEventListener("click", openChmodModal);
  $("chmod-cancel").addEventListener("click", closeChmodModal);
  $("chown-btn").addEventListener("click", openChownModal);
  $("chown-cancel").addEventListener("click", closeChownModal);

  $("add-user-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const name = $("add-user-name").value.trim();
    const isAgent = $("add-user-agent").checked;
    if (!name) return;
    await createUser(name, isAgent);
    $("add-user-name").value = "";
    $("add-user-agent").checked = false;
    userNameCache = null;
  });

  $("chmod-form").addEventListener("submit", (e) => {
    e.preventDefault();
    applyChmod($("chmod-mode").value.trim());
  });

  $("chown-form").addEventListener("submit", (e) => {
    e.preventDefault();
    applyChown($("chown-owner").value.trim(), $("chown-group").value.trim());
  });

  $("bootstrap-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const username = $("bootstrap-username").value.trim();
    if (!username) return;
    try {
      const result = await tryBootstrap(username);
      $("bootstrap-token").textContent = result.token;
      $("bootstrap-form").hidden = true;
      $("bootstrap-result").hidden = false;
      setToken(result.token, result.username);
    } catch (err) {
      toast(`bootstrap: ${err.message}`, true);
    }
  });

  $("bootstrap-continue").addEventListener("click", async () => {
    hideBackdrop();
    await loadWorkspace();
  });

  $("bootstrap-copy").addEventListener("click", (e) => {
    copyToClipboard($("bootstrap-token").textContent, e.currentTarget);
  });

  $("new-token-copy").addEventListener("click", (e) => {
    copyToClipboard($("new-token-value").textContent, e.currentTarget);
  });

  $("login-form").addEventListener("submit", async (e) => {
    e.preventDefault();
    const token = $("login-token").value.trim();
    const username = $("login-username").value.trim();
    const onBehalfOf = $("login-on-behalf-of").value.trim();
    try {
      setOnBehalfOf(onBehalfOf || null);
      if (token) {
        setToken(token);
        await refreshWhoAmI();
        if (!state.user || !state.user.authenticated) {
          throw new Error("invalid token");
        }
      } else if (username) {
        await tryLoginUsername(username);
      } else {
        throw new Error("provide a token or username");
      }
      hideBackdrop();
      await loadWorkspace();
    } catch (err) {
      setToken(null);
      setOnBehalfOf(null);
      toast(`login: ${err.message}`, true);
    }
  });
}

async function loadWorkspace() {
  await refreshWhoAmI();
  await refreshHeader();
  await renderTree();
  await refreshCommits();
}

// ---------- admin panel ----------

function openUsersPanel() {
  $("modal-backdrop").hidden = false;
  $("users-modal").hidden = false;
  $("bootstrap-modal").hidden = true;
  $("login-modal").hidden = true;
  $("new-token-display").hidden = true;
  refreshUsersTable();
}

function closeUsersPanel() {
  $("users-modal").hidden = true;
  hideBackdrop();
}

async function refreshUsersTable() {
  try {
    const res = await api("GET", "/admin/users");
    const data = await res.json();
    const tbody = $("users-tbody");
    tbody.innerHTML = "";
    for (const u of data.users || []) {
      const tr = document.createElement("tr");
      tr.innerHTML = `
        <td class="muted">${u.uid}</td>
        <td><strong></strong> <span class="muted small"></span></td>
        <td class="user-groups"></td>
        <td class="${u.has_token ? "has-token" : "no-token"}"></td>
        <td class="actions-cell"></td>
      `;
      tr.querySelector("strong").textContent = u.name;
      if (u.is_agent) tr.querySelector(".small").textContent = "(agent)";
      tr.querySelector(".user-groups").textContent = u.groups.join(", ");
      tr.querySelector("td.has-token, td.no-token").textContent = u.has_token ? "✓" : "—";

      const actions = tr.querySelector(".actions-cell");
      const tokBtn = document.createElement("button");
      tokBtn.textContent = u.has_token ? "regen token" : "issue token";
      tokBtn.addEventListener("click", () => issueToken(u.name));
      actions.append(tokBtn);

      const grpBtn = document.createElement("button");
      grpBtn.textContent = "+group";
      grpBtn.addEventListener("click", () => addUserToGroup(u.name));
      actions.append(grpBtn);

      if (u.uid !== 0) {
        const delBtn = document.createElement("button");
        delBtn.textContent = "delete";
        delBtn.className = "danger";
        delBtn.addEventListener("click", () => deleteUser(u.name));
        actions.append(delBtn);
      }
      tbody.append(tr);
    }
  } catch (e) {
    toast(`users: ${e.message}`, true);
  }
}

async function createUser(name, isAgent) {
  try {
    const res = await api("POST", "/admin/users", {
      json: { name, is_agent: isAgent },
    });
    const data = await res.json();
    if (data.token) showNewToken(data.name, data.token);
    await refreshUsersTable();
    return data;
  } catch (e) {
    toast(`add user: ${e.message}`, true);
  }
}

async function issueToken(name) {
  try {
    const res = await api("POST", `/admin/users/${encodeURIComponent(name)}/tokens`);
    const data = await res.json();
    showNewToken(data.name, data.token);
    await refreshUsersTable();
  } catch (e) {
    toast(`token: ${e.message}`, true);
  }
}

async function deleteUser(name) {
  if (!confirm(`Delete user '${name}' and all their data?`)) return;
  try {
    await api("DELETE", `/admin/users/${encodeURIComponent(name)}`);
    await refreshUsersTable();
    toast(`deleted ${name}`);
  } catch (e) {
    toast(`delete: ${e.message}`, true);
  }
}

async function addUserToGroup(name) {
  const group = prompt(`Add ${name} to which group?`);
  if (!group) return;
  try {
    await api(
      "POST",
      `/admin/users/${encodeURIComponent(name)}/groups/${encodeURIComponent(group)}`,
    );
    await refreshUsersTable();
    toast(`added ${name} to ${group}`);
  } catch (e) {
    toast(`group: ${e.message}`, true);
  }
}

function showNewToken(name, token) {
  $("new-token-user").textContent = name;
  $("new-token-value").textContent = token;
  $("new-token-display").hidden = false;
}

// ---------- chmod / chown ----------

function modeBitsFromOctal(s) {
  const cleaned = s.trim();
  if (!/^0?[0-7]{3,4}$/.test(cleaned)) return null;
  return parseInt(cleaned, 8);
}

function openChmodModal() {
  if (!state.selected) return;
  $("modal-backdrop").hidden = false;
  $("chmod-modal").hidden = false;
  $("chmod-path").textContent = state.selected;
  $("chmod-mode").value = state.selectedMode || "0644";
  $("chmod-mode").focus();
}

function closeChmodModal() {
  $("chmod-modal").hidden = true;
  hideBackdrop();
}

function openChownModal() {
  if (!state.selected) return;
  $("modal-backdrop").hidden = false;
  $("chown-modal").hidden = false;
  $("chown-path").textContent = state.selected;
  $("chown-owner").value = state.selectedOwner || "";
  $("chown-group").value = "";
  $("chown-owner").focus();
}

function closeChownModal() {
  $("chown-modal").hidden = true;
  hideBackdrop();
}

async function applyChmod(modeStr) {
  if (!state.selected) return;
  try {
    await api("POST", `/admin/chmod/${encodeSegments(state.selected)}`, {
      json: { mode: modeStr },
    });
    toast(`chmod ${modeStr} ${state.selected}`);
    closeChmodModal();
    await openFile(state.selected, document.querySelector(".tree-item.selected"));
  } catch (e) {
    toast(`chmod: ${e.message}`, true);
  }
}

async function applyChown(owner, group) {
  if (!state.selected) return;
  try {
    await api("POST", `/admin/chown/${encodeSegments(state.selected)}`, {
      json: { owner, group: group || null },
    });
    toast(`chown ${owner} ${state.selected}`);
    closeChownModal();
    await openFile(state.selected, document.querySelector(".tree-item.selected"));
  } catch (e) {
    toast(`chown: ${e.message}`, true);
  }
}

async function init() {
  bind();
  // Restore username-auth if no token
  const savedUser = localStorage.getItem(USER_KEY);
  if (!state.token && savedUser) {
    state.usernameAuth = savedUser;
  }

  let health = null;
  try {
    const res = await api("GET", "/health", { skipAuth: true });
    health = await res.json();
  } catch (e) {
    $("counters").textContent = "(server unreachable)";
    return;
  }

  if (health.needs_bootstrap) {
    showBackdrop("bootstrap");
    return;
  }

  // Validate any stored credential
  if (state.token || state.usernameAuth) {
    try {
      const w = await api("GET", "/auth/whoami");
      const wj = await w.json();
      if (!wj.authenticated && state.token) {
        // token rejected
        setToken(null);
        state.usernameAuth = null;
        showBackdrop("login");
        return;
      }
    } catch (e) {
      setToken(null);
      state.usernameAuth = null;
      showBackdrop("login");
      return;
    }
  } else {
    // No credential at all — show login (anon-root would still work but we want explicit identity)
    showBackdrop("login");
    return;
  }

  await loadWorkspace();
}

init();
