use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Notify, RwLock};

use crate::auth::session::Session;
use crate::auth::{Gid, Uid};
use crate::config::Config;
use crate::error::VfsError;
use crate::fs::{FsOptions, GrepResult, HandleId, LsEntry, StatInfo, VirtualFs};
use crate::persist::{
    LocalStateBackend, MemoryPersistenceBackend, PersistenceBackend, PersistenceInfo,
};
use crate::store::commit::CommitObject;
use crate::vcs::Vcs;

struct DbInner {
    fs: VirtualFs,
    vcs: Vcs,
}

/// Thread-safe, concurrent markdown database.
///
/// All methods take `&self` (not `&mut self`). The struct is `Clone`
/// via the inner `Arc`, so it can be shared across threads cheaply.
#[derive(Clone)]
pub struct MarkdownDb {
    inner: Arc<RwLock<DbInner>>,
    persist: Arc<dyn PersistenceBackend>,
    config: Arc<Config>,
    write_count: Arc<AtomicU64>,
    save_notify: Arc<Notify>,
}

impl MarkdownDb {
    pub fn open(config: Config) -> Result<Self, VfsError> {
        let persist: Arc<dyn PersistenceBackend> = Arc::new(LocalStateBackend::new(&config.data_dir));
        Ok(Self::open_with_backend(config, persist))
    }

    pub fn open_with_backend(config: Config, persist: Arc<dyn PersistenceBackend>) -> Self {
        let options = FsOptions {
            compatibility_target: config.compatibility_target,
        };
        let (fs, vcs) = persist
            .load()
            .ok()
            .flatten()
            .map(|state| (state.fs, state.vcs))
            .unwrap_or_else(|| (VirtualFs::new_with_options(options), Vcs::new()));

        MarkdownDb {
            inner: Arc::new(RwLock::new(DbInner { fs, vcs })),
            persist,
            config: Arc::new(config),
            write_count: Arc::new(AtomicU64::new(0)),
            save_notify: Arc::new(Notify::new()),
        }
    }

    pub fn open_memory() -> Self {
        let config = Config::from_env();
        let options = FsOptions {
            compatibility_target: config.compatibility_target,
        };
        MarkdownDb {
            inner: Arc::new(RwLock::new(DbInner {
                fs: VirtualFs::new_with_options(options),
                vcs: Vcs::new(),
            })),
            persist: Arc::new(MemoryPersistenceBackend),
            config: Arc::new(config),
            write_count: Arc::new(AtomicU64::new(0)),
            save_notify: Arc::new(Notify::new()),
        }
    }

    fn mark_dirty(&self) {
        let count = self.write_count.fetch_add(1, Ordering::Relaxed);
        if count + 1 >= self.config.auto_save_write_threshold {
            self.save_notify.notify_one();
        }
    }

    /// Spawn a background auto-save task. Returns a handle that can be aborted on shutdown.
    pub fn spawn_auto_save(&self) -> tokio::task::JoinHandle<()> {
        let db = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(
                        db.config.auto_save_interval_secs,
                    )) => {}
                    _ = db.save_notify.notified() => {}
                }

                let prev = db.write_count.swap(0, Ordering::Relaxed);
                if prev > 0 {
                    if let Err(e) = db.save().await {
                        tracing::error!("auto-save failed: {e}");
                    } else {
                        tracing::debug!("auto-saved after {prev} writes");
                    }
                }
            }
        })
    }

    pub async fn save(&self) -> Result<(), VfsError> {
        let guard = self.inner.read().await;
        self.persist.save(&guard.fs, &guard.vcs)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn persist_info(&self) -> PersistenceInfo {
        self.persist.info()
    }

    // ─── Read operations (take read lock) ───

    pub async fn cat(&self, path: &str) -> Result<Vec<u8>, VfsError> {
        let guard = self.inner.read().await;
        guard.fs.cat_owned(path)
    }

    pub async fn ls(&self, path: Option<&str>) -> Result<Vec<LsEntry>, VfsError> {
        let guard = self.inner.read().await;
        guard.fs.ls(path)
    }

    pub async fn stat(&self, path: &str) -> Result<StatInfo, VfsError> {
        let guard = self.inner.read().await;
        guard.fs.stat(path)
    }

    pub async fn pwd(&self) -> String {
        let guard = self.inner.read().await;
        guard.fs.pwd()
    }

    pub async fn tree(
        &self,
        path: Option<&str>,
        session: Option<&Session>,
    ) -> Result<String, VfsError> {
        let guard = self.inner.read().await;
        guard.fs.tree(path, "", session)
    }

    pub async fn find(
        &self,
        path: Option<&str>,
        pattern: Option<&str>,
        session: Option<&Session>,
    ) -> Result<Vec<String>, VfsError> {
        let guard = self.inner.read().await;
        guard.fs.find(path, pattern, session)
    }

    pub async fn grep(
        &self,
        pattern: &str,
        path: Option<&str>,
        recursive: bool,
        session: Option<&Session>,
    ) -> Result<Vec<GrepResult>, VfsError> {
        let guard = self.inner.read().await;
        guard.fs.grep(pattern, path, recursive, session)
    }

    pub async fn vcs_log(&self) -> Vec<CommitObject> {
        let guard = self.inner.read().await;
        guard.vcs.log().into_iter().cloned().collect()
    }

    pub async fn vcs_status(&self) -> Result<String, VfsError> {
        let guard = self.inner.read().await;
        let inner = &*guard;
        inner.vcs.status(&inner.fs)
    }

    // ─── Write operations (take write lock) ───

    pub async fn touch(&self, path: &str, uid: Uid, gid: Gid) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.touch(path, uid, gid)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn write_file(&self, path: &str, content: Vec<u8>) -> Result<(), VfsError> {
        if content.len() > self.config.max_file_size {
            return Err(VfsError::InvalidArgs {
                message: format!(
                    "file size {} exceeds max {}",
                    content.len(),
                    self.config.max_file_size
                ),
            });
        }
        let mut guard = self.inner.write().await;
        guard.fs.write_file(path, content)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn mkdir(&self, path: &str, uid: Uid, gid: Gid) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.mkdir(path, uid, gid)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn mkdir_p(&self, path: &str, uid: Uid, gid: Gid) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.mkdir_p(path, uid, gid)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn rm(&self, path: &str) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.rm(path)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn rm_rf(&self, path: &str) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.rm_rf(path)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn mv(&self, src: &str, dst: &str) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.mv(src, dst)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn cp(
        &self,
        src: &str,
        dst: &str,
        uid: Uid,
        gid: Gid,
    ) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.cp(src, dst, uid, gid)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn chmod(&self, path: &str, mode: u16) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.chmod(path, mode)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn chown(&self, path: &str, uid: Uid, gid: Gid) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.chown(path, uid, gid)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn ln_s(
        &self,
        target: &str,
        link_path: &str,
        uid: Uid,
        gid: Gid,
    ) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.ln_s(target, link_path, uid, gid)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn link(&self, target: &str, link_path: &str) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.link(target, link_path)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn readlink(&self, path: &str) -> Result<String, VfsError> {
        let guard = self.inner.read().await;
        guard.fs.readlink(path)
    }

    pub async fn truncate(&self, path: &str, size: usize) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.truncate(path, size)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn read_file_at(
        &self,
        path: &str,
        offset: usize,
        size: usize,
    ) -> Result<Vec<u8>, VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.read_file_at(path, offset, size)
    }

    pub async fn write_file_at(
        &self,
        path: &str,
        offset: usize,
        data: &[u8],
    ) -> Result<usize, VfsError> {
        let end = offset.saturating_add(data.len());
        if end > self.config.max_file_size {
            return Err(VfsError::InvalidArgs {
                message: format!("write exceeds max file size {}", self.config.max_file_size),
            });
        }
        let mut guard = self.inner.write().await;
        let written = guard.fs.write_file_at(path, offset, data)?;
        drop(guard);
        self.mark_dirty();
        Ok(written)
    }

    pub async fn open_file(&self, path: &str, writable: bool) -> Result<HandleId, VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.open(path, writable)
    }

    pub async fn open_dir(&self, path: &str) -> Result<HandleId, VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.opendir(path)
    }

    pub async fn read_handle(&self, handle: HandleId, size: usize) -> Result<Vec<u8>, VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.read_handle(handle, size)
    }

    pub async fn write_handle(&self, handle: HandleId, data: &[u8]) -> Result<usize, VfsError> {
        let mut guard = self.inner.write().await;
        let written = guard.fs.write_handle(handle, data)?;
        drop(guard);
        self.mark_dirty();
        Ok(written)
    }

    pub async fn release_handle(&self, handle: HandleId) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        guard.fs.release_handle(handle)
    }

    pub async fn commit(&self, message: &str, author: &str) -> Result<String, VfsError> {
        let mut guard = self.inner.write().await;
        let inner = &mut *guard;
        let id = inner.vcs.commit(&inner.fs, message, author)?;
        drop(guard);
        self.mark_dirty();
        Ok(id.short_hex())
    }

    pub async fn revert(&self, hash_prefix: &str) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        let inner = &mut *guard;
        inner.vcs.revert(&mut inner.fs, hash_prefix)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    // ─── Command execution (write lock — dispatches through cmd module) ───

    pub async fn execute_command(
        &self,
        line: &str,
        session: &mut Session,
    ) -> Result<String, VfsError> {
        use crate::cmd;
        use crate::cmd::parser;

        let pipeline = parser::parse_pipeline(line);
        if pipeline.commands.is_empty() {
            return Ok(String::new());
        }

        if let Some(first) = pipeline.commands.first() {
            match first.program.as_str() {
                "commit" => {
                    let msg = if first.args.is_empty() {
                        "snapshot"
                    } else {
                        &first.args.join(" ")
                    };
                    let hash = self.commit(msg, &session.username).await?;
                    return Ok(format!("[{hash}] {msg}\n"));
                }
                "log" => {
                    let commits = self.vcs_log().await;
                    if commits.is_empty() {
                        return Ok("No commits yet.\n".to_string());
                    }
                    let mut output = String::new();
                    for c in &commits {
                        let time = chrono::DateTime::from_timestamp(c.timestamp as i64, 0)
                            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| "???".to_string());
                        output.push_str(&format!(
                            "{} {} {} {}\n",
                            c.id.short_hex(),
                            time,
                            c.author,
                            c.message
                        ));
                    }
                    return Ok(output);
                }
                "revert" => {
                    if first.args.is_empty() {
                        return Err(VfsError::InvalidArgs {
                            message: "revert: need commit hash prefix".to_string(),
                        });
                    }
                    self.revert(&first.args[0]).await?;
                    return Ok(format!("Reverted to {}\n", first.args[0]));
                }
                "status" => {
                    return self.vcs_status().await;
                }
                _ => {}
            }
        }

        let mut guard = self.inner.write().await;
        let inner = &mut *guard;
        let result = cmd::execute_pipeline(&pipeline, &mut inner.fs, session);
        let is_write = pipeline.commands.iter().any(|c| {
            matches!(
                c.program.as_str(),
                "touch"
                    | "write"
                    | "mkdir"
                    | "rm"
                    | "rmdir"
                    | "mv"
                    | "cp"
                    | "chmod"
                    | "chown"
                    | "ln"
                    | "adduser"
                    | "addagent"
                    | "deluser"
                    | "addgroup"
                    | "delgroup"
                    | "usermod"
            )
        });
        drop(guard);
        if is_write {
            self.mark_dirty();
        }
        result
    }

    // ─── Auth helpers ───

    pub async fn login(&self, username: &str) -> Result<Session, VfsError> {
        let guard = self.inner.read().await;
        let uid = guard
            .registry_lookup_uid(username)
            .ok_or_else(|| VfsError::AuthError {
                message: format!("unknown user: {username}"),
            })?;
        let user = guard
            .registry_get_user(uid)
            .ok_or_else(|| VfsError::AuthError {
                message: format!("user uid={uid} not found"),
            })?;
        let session = Session::new(
            user.uid,
            user.groups.first().copied().unwrap_or(0),
            user.groups.clone(),
            user.name.clone(),
        );

        // Note: do not cd into the user's home here. fs.cwd is global state
        // shared across requests; mutating it from login() would shift every
        // other in-flight request's path resolution. The CLI does its own cd
        // post-login via cd_to_home below.
        Ok(session)
    }

    /// CLI helper: change the global cwd to the given user's home dir,
    /// if it exists. Safe only in single-tenant usages (CLI/REPL).
    pub async fn cd_to_home(&self, username: &str) {
        let mut guard = self.inner.write().await;
        let home = format!("/home/{username}");
        if guard.fs.stat(&home).is_ok() {
            let _ = guard.fs.cd(&home);
        }
    }

    pub async fn authenticate_token(&self, raw_token: &str) -> Result<Session, VfsError> {
        let guard = self.inner.read().await;
        let uid = guard
            .fs
            .registry
            .authenticate_token(raw_token)
            .ok_or_else(|| VfsError::AuthError {
                message: "invalid token".to_string(),
            })?;
        let user = guard
            .fs
            .registry
            .get_user(uid)
            .ok_or_else(|| VfsError::AuthError {
                message: "token user not found".to_string(),
            })?;
        Ok(Session::new(
            user.uid,
            user.groups.first().copied().unwrap_or(0),
            user.groups.clone(),
            user.name.clone(),
        ))
    }

    pub async fn has_users(&self) -> bool {
        let guard = self.inner.read().await;
        guard
            .fs
            .registry
            .list_users()
            .iter()
            .any(|u| u.uid != crate::auth::ROOT_UID)
    }

    pub async fn create_admin(&self, name: &str) -> Result<Session, VfsError> {
        let mut guard = self.inner.write().await;
        let (uid, _) = guard.fs.registry.add_user(name, false)?;
        let _ = guard.fs.registry.usermod_add_group(name, "wheel");
        let user = guard.fs.registry.get_user(uid).unwrap();
        let gid = user.groups.first().copied().unwrap_or(0);
        let session = Session::new(uid, gid, user.groups.clone(), user.name.clone());

        let _ = guard.fs.mkdir_p(
            "/home",
            crate::auth::ROOT_UID,
            crate::auth::ROOT_GID,
        );
        let home_path = format!("/home/{name}");
        let _ = guard.fs.mkdir(&home_path, uid, gid);
        // Lock the home dir down: only the owner can list/enter it.
        let _ = guard.fs.chmod(&home_path, 0o700);

        drop(guard);
        self.mark_dirty();
        Ok(session)
    }

    pub async fn commit_count(&self) -> usize {
        let guard = self.inner.read().await;
        guard.vcs.commits.len()
    }

    pub async fn object_count(&self) -> usize {
        let guard = self.inner.read().await;
        guard.vcs.store.object_count()
    }

    pub async fn inode_count(&self) -> usize {
        let guard = self.inner.read().await;
        guard.fs.all_inodes().len()
    }

    pub fn snapshot_fs(&self) -> VirtualFs {
        self.inner.blocking_read().fs.clone()
    }

    pub async fn lookup_gid(&self, name: &str) -> Option<Gid> {
        self.inner.read().await.fs.registry.lookup_gid(name)
    }

    /// Return (is_wheel_member, is_agent) for a uid.
    pub async fn principal_flags(&self, uid: Uid) -> (bool, bool) {
        let guard = self.inner.read().await;
        let is_wheel = guard.fs.registry.is_wheel_member(uid);
        let is_agent = guard
            .fs
            .registry
            .get_user(uid)
            .map(|u| u.is_agent)
            .unwrap_or(false);
        (is_wheel, is_agent)
    }

    // ---------- Admin operations (gated on wheel/root) ----------

    fn require_wheel(&self, session: &Session, registry: &crate::auth::registry::UserRegistry) -> Result<(), VfsError> {
        if session.is_effectively_root() || registry.is_wheel_member(session.uid) {
            Ok(())
        } else {
            Err(VfsError::PermissionDenied {
                path: "admin".to_string(),
            })
        }
    }

    pub async fn admin_list_users(&self, session: &Session) -> Result<Vec<UserSummary>, VfsError> {
        let guard = self.inner.read().await;
        self.require_wheel(session, &guard.fs.registry)?;
        Ok(guard
            .fs
            .registry
            .list_users()
            .into_iter()
            .map(|u| UserSummary {
                uid: u.uid,
                name: u.name.clone(),
                groups: u
                    .groups
                    .iter()
                    .filter_map(|g| guard.fs.registry.group_name(*g).map(|s| s.to_string()))
                    .collect(),
                is_agent: u.is_agent,
                has_token: u.api_token.is_some(),
            })
            .collect())
    }

    pub async fn admin_list_groups(&self, session: &Session) -> Result<Vec<GroupSummary>, VfsError> {
        let guard = self.inner.read().await;
        self.require_wheel(session, &guard.fs.registry)?;
        Ok(guard
            .fs
            .registry
            .list_groups()
            .into_iter()
            .map(|g| GroupSummary {
                gid: g.gid,
                name: g.name.clone(),
                members: g
                    .members
                    .iter()
                    .filter_map(|u| guard.fs.registry.user_name(*u).map(|s| s.to_string()))
                    .collect(),
            })
            .collect())
    }

    pub async fn admin_add_user(
        &self,
        session: &Session,
        name: &str,
        is_agent: bool,
    ) -> Result<(Uid, Option<String>), VfsError> {
        let mut guard = self.inner.write().await;
        self.require_wheel(session, &guard.fs.registry)?;
        let result = guard.fs.registry.add_user(name, is_agent)?;
        let uid = result.0;
        let gid = guard
            .fs
            .registry
            .get_user(uid)
            .map(|u| u.groups[0])
            .unwrap_or(0);
        let home = format!("/home/{name}");
        let _ = guard.fs.mkdir_p(&home, uid, gid);
        // Lock the home dir down: only the owner can list/enter it.
        let _ = guard.fs.chmod(&home, 0o700);
        drop(guard);
        self.mark_dirty();
        Ok(result)
    }

    pub async fn admin_del_user(&self, session: &Session, name: &str) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        self.require_wheel(session, &guard.fs.registry)?;
        guard.fs.registry.del_user(name)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn admin_add_group(&self, session: &Session, name: &str) -> Result<Gid, VfsError> {
        let mut guard = self.inner.write().await;
        self.require_wheel(session, &guard.fs.registry)?;
        let gid = guard.fs.registry.add_group(name)?;
        drop(guard);
        self.mark_dirty();
        Ok(gid)
    }

    pub async fn admin_del_group(&self, session: &Session, name: &str) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        self.require_wheel(session, &guard.fs.registry)?;
        guard.fs.registry.del_group(name)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn admin_usermod_add(
        &self,
        session: &Session,
        user: &str,
        group: &str,
    ) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        self.require_wheel(session, &guard.fs.registry)?;
        guard.fs.registry.usermod_add_group(user, group)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn admin_usermod_remove(
        &self,
        session: &Session,
        user: &str,
        group: &str,
    ) -> Result<(), VfsError> {
        let mut guard = self.inner.write().await;
        self.require_wheel(session, &guard.fs.registry)?;
        guard.fs.registry.usermod_remove_group(user, group)?;
        drop(guard);
        self.mark_dirty();
        Ok(())
    }

    pub async fn admin_issue_token(&self, session: &Session, name: &str) -> Result<String, VfsError> {
        let mut guard = self.inner.write().await;
        self.require_wheel(session, &guard.fs.registry)?;
        let raw = guard.fs.registry.regenerate_token(name)?;
        drop(guard);
        self.mark_dirty();
        Ok(raw)
    }

    pub async fn admin_chmod(
        &self,
        session: &Session,
        path: &str,
        mode: u16,
    ) -> Result<(), VfsError> {
        let info = self.stat(path).await?;
        let is_wheel = {
            let guard = self.inner.read().await;
            guard.fs.registry.is_wheel_member(session.uid)
        };
        if !session.is_effectively_root()
            && !is_wheel
            && !session.is_effective_owner(info.uid)
        {
            return Err(VfsError::PermissionDenied {
                path: path.to_string(),
            });
        }
        self.chmod(path, mode).await
    }

    pub async fn admin_chown(
        &self,
        session: &Session,
        path: &str,
        owner: &str,
        group: Option<&str>,
    ) -> Result<(), VfsError> {
        let guard = self.inner.read().await;
        // chown across owners requires root; same-owner gid change requires ownership
        let target_uid = guard
            .fs
            .registry
            .lookup_uid(owner)
            .ok_or_else(|| VfsError::AuthError {
                message: format!("no such user: {owner}"),
            })?;
        let target_gid = match group {
            Some(g) => guard
                .fs
                .registry
                .lookup_gid(g)
                .ok_or_else(|| VfsError::AuthError {
                    message: format!("no such group: {g}"),
                })?,
            None => guard
                .fs
                .registry
                .get_user(target_uid)
                .map(|u| u.groups[0])
                .unwrap_or(0),
        };
        let is_wheel = guard.fs.registry.is_wheel_member(session.uid);
        drop(guard);
        let info = self.stat(path).await?;
        let is_owner_change = info.uid != target_uid;
        if is_owner_change && !session.is_effectively_root() && !is_wheel {
            return Err(VfsError::PermissionDenied {
                path: path.to_string(),
            });
        }
        if !is_owner_change
            && !session.is_effectively_root()
            && !is_wheel
            && !session.is_effective_owner(info.uid)
        {
            return Err(VfsError::PermissionDenied {
                path: path.to_string(),
            });
        }
        self.chown(path, target_uid, target_gid).await
    }
}

#[derive(serde::Serialize)]
pub struct UserSummary {
    pub uid: Uid,
    pub name: String,
    pub groups: Vec<String>,
    pub is_agent: bool,
    pub has_token: bool,
}

#[derive(serde::Serialize)]
pub struct GroupSummary {
    pub gid: Gid,
    pub name: String,
    pub members: Vec<String>,
}

impl DbInner {
    fn registry_lookup_uid(&self, name: &str) -> Option<Uid> {
        self.fs.registry.lookup_uid(name)
    }

    fn registry_get_user(&self, uid: Uid) -> Option<crate::auth::User> {
        self.fs.registry.get_user(uid).cloned()
    }
}
