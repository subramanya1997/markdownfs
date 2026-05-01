use std::sync::Arc;

use rmcp::model::*;
use rmcp::service::RoleServer;
use rmcp::{ErrorData as McpError, ServerHandler};

use crate::auth::perms::Access;
use crate::auth::session::Session;
use crate::db::MarkdownDb;

#[derive(Clone)]
pub struct McpServer {
    db: MarkdownDb,
    session: Session,
}

impl McpServer {
    pub fn new(db: MarkdownDb) -> Self {
        Self {
            db,
            session: Session::root(),
        }
    }

    pub fn with_session(db: MarkdownDb, session: Session) -> Self {
        Self { db, session }
    }

    async fn require_read(&self, path: &str) -> Result<(), String> {
        if self.session.is_effectively_root() {
            return Ok(());
        }
        let info = self.db.stat(path).await.map_err(|e| e.to_string())?;
        if self
            .session
            .has_permission_bits(info.mode, info.uid, info.gid, Access::Read)
        {
            Ok(())
        } else {
            Err(format!("permission denied: {path}"))
        }
    }

    async fn require_write(&self, path: &str) -> Result<(), String> {
        if self.session.is_effectively_root() {
            return Ok(());
        }
        match self.db.stat(path).await {
            Ok(info) => {
                if self
                    .session
                    .has_permission_bits(info.mode, info.uid, info.gid, Access::Write)
                {
                    Ok(())
                } else {
                    Err(format!("permission denied: {path}"))
                }
            }
            Err(_) => self.require_parent_write(path).await,
        }
    }

    async fn require_parent_write(&self, path: &str) -> Result<(), String> {
        if self.session.is_effectively_root() {
            return Ok(());
        }
        let parent = parent_of(path);
        let parent_path = if parent.is_empty() { "/" } else { parent.as_str() };
        let info = self.db.stat(parent_path).await.map_err(|e| e.to_string())?;
        if self
            .session
            .has_permission_bits(info.mode, info.uid, info.gid, Access::Write)
        {
            Ok(())
        } else {
            Err(format!("permission denied: {parent_path}"))
        }
    }

    fn tool_defs() -> Vec<Tool> {
        vec![
            make_tool("read_file", "Read a markdown file by path", serde_json::json!({
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            })),
            make_tool("write_file", "Write content to a markdown file (creates if needed)", serde_json::json!({
                "properties": {"path": {"type": "string"}, "content": {"type": "string"}},
                "required": ["path", "content"]
            })),
            make_tool("list_directory", "List files in a directory", serde_json::json!({
                "properties": {"path": {"type": "string"}}
            })),
            make_tool("search_files", "Search file contents with a regex pattern", serde_json::json!({
                "properties": {"pattern": {"type": "string"}, "path": {"type": "string"}, "recursive": {"type": "boolean"}},
                "required": ["pattern"]
            })),
            make_tool("find_files", "Find files by glob pattern", serde_json::json!({
                "properties": {"path": {"type": "string"}, "name": {"type": "string"}}
            })),
            make_tool("create_directory", "Create a directory (with parents)", serde_json::json!({
                "properties": {"path": {"type": "string"}},
                "required": ["path"]
            })),
            make_tool("delete_file", "Delete a file or directory", serde_json::json!({
                "properties": {"path": {"type": "string"}, "recursive": {"type": "boolean"}},
                "required": ["path"]
            })),
            make_tool("move_file", "Move or rename a file/directory", serde_json::json!({
                "properties": {"source": {"type": "string"}, "destination": {"type": "string"}},
                "required": ["source", "destination"]
            })),
            make_tool("commit", "Commit current filesystem state", serde_json::json!({
                "properties": {"message": {"type": "string"}},
                "required": ["message"]
            })),
            make_tool("get_history", "Show commit history", serde_json::json!({
                "properties": {}
            })),
            make_tool("revert", "Revert to a previous commit", serde_json::json!({
                "properties": {"hash": {"type": "string"}},
                "required": ["hash"]
            })),
        ]
    }

    async fn handle_tool(&self, name: &str, args: &serde_json::Value) -> Result<String, String> {
        match name {
            "read_file" => {
                let path = args["path"].as_str().ok_or("missing path")?;
                self.require_read(path).await?;
                let content = self.db.cat(path).await.map_err(|e| e.to_string())?;
                Ok(String::from_utf8_lossy(&content).into_owned())
            }
            "write_file" => {
                let path = args["path"].as_str().ok_or("missing path")?;
                let content = args["content"].as_str().ok_or("missing content")?;
                self.require_write(path).await?;
                let uid = self.session.effective_uid();
                let gid = self.session.effective_gid();
                if self.db.stat(path).await.is_err() {
                    let trimmed = path.trim_end_matches('/');
                    if let Some(idx) = trimmed.rfind('/') {
                        let parent = &trimmed[..idx];
                        if !parent.is_empty() && self.db.stat(parent).await.is_err() {
                            self.db
                                .mkdir_p(parent, uid, gid)
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                    }
                    self.db.touch(path, uid, gid).await.map_err(|e| e.to_string())?;
                }
                self.db
                    .write_file(path, content.as_bytes().to_vec())
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!("Written {} bytes to {path}", content.len()))
            }
            "list_directory" => {
                let path = args.get("path").and_then(|v| v.as_str());
                if let Some(p) = path {
                    self.require_read(p).await?;
                }
                let entries = self.db.ls(path).await.map_err(|e| e.to_string())?;
                let mut out = String::new();
                for e in &entries {
                    if !self.session.is_effectively_root()
                        && !self
                            .session
                            .has_permission_bits(e.mode, e.uid, e.gid, Access::Read)
                    {
                        continue;
                    }
                    let suffix = if e.is_dir { "/" } else { "" };
                    out.push_str(&format!("{}{suffix}\n", e.name));
                }
                Ok(out)
            }
            "search_files" => {
                let pattern = args["pattern"].as_str().ok_or("missing pattern")?;
                let path = args.get("path").and_then(|v| v.as_str());
                let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(true);
                let results = self
                    .db
                    .grep(pattern, path, recursive, Some(&self.session))
                    .await
                    .map_err(|e| e.to_string())?;
                let mut out = String::new();
                for r in &results {
                    out.push_str(&format!("{}:{}: {}\n", r.file, r.line_num, r.line));
                }
                if out.is_empty() {
                    out = "No matches found.".to_string();
                }
                Ok(out)
            }
            "find_files" => {
                let path = args.get("path").and_then(|v| v.as_str());
                let name = args.get("name").and_then(|v| v.as_str());
                let results = self
                    .db
                    .find(path, name, Some(&self.session))
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(results.join("\n"))
            }
            "create_directory" => {
                let path = args["path"].as_str().ok_or("missing path")?;
                self.require_parent_write(path).await?;
                let uid = self.session.effective_uid();
                let gid = self.session.effective_gid();
                self.db.mkdir_p(path, uid, gid).await.map_err(|e| e.to_string())?;
                Ok(format!("Created directory: {path}"))
            }
            "delete_file" => {
                let path = args["path"].as_str().ok_or("missing path")?;
                let recursive = args.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);
                self.require_parent_write(path).await?;
                if recursive {
                    self.db.rm_rf(path).await
                } else {
                    self.db.rm(path).await
                }
                .map_err(|e| e.to_string())?;
                Ok(format!("Deleted: {path}"))
            }
            "move_file" => {
                let src = args["source"].as_str().ok_or("missing source")?;
                let dst = args["destination"].as_str().ok_or("missing destination")?;
                self.require_parent_write(src).await?;
                self.require_parent_write(dst).await?;
                self.db.mv(src, dst).await.map_err(|e| e.to_string())?;
                Ok(format!("Moved {src} -> {dst}"))
            }
            "commit" => {
                let message = args["message"].as_str().ok_or("missing message")?;
                let author = if self.session.username.is_empty() {
                    "mcp-agent".to_string()
                } else {
                    self.session.username.clone()
                };
                let hash = self
                    .db
                    .commit(message, &author)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(format!("[{hash}] {message}"))
            }
            "get_history" => {
                let commits = self.db.vcs_log().await;
                if commits.is_empty() { return Ok("No commits yet.".to_string()); }
                let mut out = String::new();
                for c in &commits { out.push_str(&format!("{} {} {}\n", c.id.short_hex(), c.author, c.message)); }
                Ok(out)
            }
            "revert" => {
                let hash = args["hash"].as_str().ok_or("missing hash")?;
                self.db.revert(hash).await.map_err(|e| e.to_string())?;
                Ok(format!("Reverted to {hash}"))
            }
            _ => Err(format!("unknown tool: {name}")),
        }
    }
}

fn parent_of(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(idx) => trimmed[..idx].to_string(),
        None => String::new(),
    }
}

fn tool_schema(props: serde_json::Value) -> Arc<JsonObject> {
    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), "object".into());
    if let Some(p) = props.get("properties") {
        obj.insert("properties".into(), p.clone());
    }
    if let Some(r) = props.get("required") {
        obj.insert("required".into(), r.clone());
    }
    Arc::new(obj.into())
}

fn make_tool(name: &'static str, desc: &'static str, schema: serde_json::Value) -> Tool {
    Tool::new(name, desc, tool_schema(schema))
}

impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        let mut caps = ServerCapabilities::default();
        caps.tools = Some(ToolsCapability { list_changed: None });
        caps.resources = Some(ResourcesCapability {
            subscribe: None,
            list_changed: None,
        });
        InitializeResult::new(caps).with_instructions(
            "mdfs is a markdown-only virtual filesystem with Git-like versioning. \
             Use tools to read, write, search, and manage markdown files. \
             All files must have .md extension.",
        )
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        async {
            let mut result = ListToolsResult::default();
            result.tools = Self::tool_defs();
            Ok(result)
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        async move {
            let args = serde_json::to_value(&request.arguments).unwrap_or_default();
            match self.handle_tool(&request.name, &args).await {
                Ok(text) => {
                    let mut result = CallToolResult::default();
                    result.content = vec![Content::text(text)];
                    result.is_error = Some(false);
                    Ok(result)
                }
                Err(e) => {
                    let mut result = CallToolResult::default();
                    result.content = vec![Content::text(e)];
                    result.is_error = Some(true);
                    Ok(result)
                }
            }
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        async {
            let resource = RawResource::new("mdfs://tree", "Directory Tree")
                .with_description("Full directory tree of the filesystem")
                .with_mime_type("text/plain");
            let legacy_resource = RawResource::new("markdownfs://tree", "Directory Tree (legacy)")
                .with_description("Legacy alias for mdfs://tree")
                .with_mime_type("text/plain");
            let mut result = ListResourcesResult::default();
            result.resources = vec![
                Annotated::new(resource, None),
                Annotated::new(legacy_resource, None),
            ];
            Ok(result)
        }
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        async move {
            let uri = request.uri.as_str();
            if uri == "mdfs://tree" || uri == "markdownfs://tree" {
                let tree = self
                    .db
                    .tree(None, None)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(ReadResourceResult::new(vec![ResourceContents::text(tree, uri)]))
            } else if let Some(path) = uri
                .strip_prefix("mdfs://files/")
                .or_else(|| uri.strip_prefix("markdownfs://files/"))
            {
                let content = self
                    .db
                    .cat(path)
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(ReadResourceResult::new(vec![ResourceContents::text(
                    String::from_utf8_lossy(&content).into_owned(),
                    uri,
                )]))
            } else {
                Err(McpError::invalid_params(format!("unknown resource: {uri}"), None))
            }
        }
    }
}
