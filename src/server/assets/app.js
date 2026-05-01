// MarkdownFS minimal web UI. Pure REST client.

const $ = (id) => document.getElementById(id);

const state = {
  expanded: new Set([""]),
  selected: null,
  editing: false,
  originalContent: "",
};

async function api(method, path, opts = {}) {
  const init = { method, headers: {} };
  if (opts.json !== undefined) {
    init.body = JSON.stringify(opts.json);
    init.headers["content-type"] = "application/json";
  } else if (opts.body !== undefined) {
    init.body = opts.body;
    init.headers["content-type"] = opts.contentType || "text/markdown";
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
    throw new Error(msg);
  }
  return res;
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
  } catch (e) {
    $("counters").textContent = "(server unreachable)";
  }
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
  } catch (e) {
    toast(`open: ${e.message}`, true);
  }
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
}

(async function init() {
  bind();
  await refreshHeader();
  await renderTree();
  await refreshCommits();
})();
