use crate::auth::session::Session;
use crate::auth::ROOT_UID;

/// Normalize a client-supplied path against the session's effective home.
///
/// Rules:
/// - Absolute paths (`/foo`) are returned unchanged.
/// - `~` and `~/...` expand to the session's effective home.
/// - Bare relative paths (`foo`, `foo/bar`) are resolved under the session's
///   effective home.
/// - The empty string maps to the home itself.
/// - For root and anonymous-root sessions there is no `/home/root`, so their
///   effective home is `/` — relative paths from those sessions resolve at
///   the root, preserving the historical behavior for scripts.
/// - When delegating, the *delegate's* username defines the effective home.
///   That's what makes "agent on behalf of alice" land in alice's home.
pub fn resolve_user_path(session: &Session, path: &str) -> String {
    if path.starts_with('/') {
        return path.to_string();
    }

    let home = effective_home(session);

    // Tilde expansion.
    if path == "~" {
        return home;
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return join_under(&home, rest);
    }

    if path.is_empty() {
        return home;
    }

    join_under(&home, path)
}

fn effective_home(session: &Session) -> String {
    // Delegate's identity defines the workspace when on-behalf-of is set.
    let username = match &session.delegate {
        Some(d) => d.username.as_str(),
        None => session.username.as_str(),
    };

    let uid = match &session.delegate {
        Some(d) => d.uid,
        None => session.uid,
    };

    // Root, the synthetic-uid group delegation, or anonymous → /
    if uid == ROOT_UID || username.is_empty() || username.starts_with(':') {
        return "/".to_string();
    }
    format!("/home/{username}")
}

fn join_under(home: &str, rest: &str) -> String {
    let trimmed = rest.trim_start_matches('/');
    if home == "/" {
        format!("/{trimmed}")
    } else {
        format!("{home}/{trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::session::{DelegateContext, Session};

    fn user(name: &str, uid: u32) -> Session {
        Session::new(uid, uid, vec![uid], name.to_string())
    }

    #[test]
    fn absolute_passthrough() {
        let s = user("alice", 1);
        assert_eq!(resolve_user_path(&s, "/foo/bar.md"), "/foo/bar.md");
    }

    #[test]
    fn relative_goes_under_home() {
        let s = user("alice", 1);
        assert_eq!(
            resolve_user_path(&s, "notes/idea.md"),
            "/home/alice/notes/idea.md"
        );
    }

    #[test]
    fn tilde_expands_to_home() {
        let s = user("alice", 1);
        assert_eq!(resolve_user_path(&s, "~"), "/home/alice");
        assert_eq!(resolve_user_path(&s, "~/notes/idea.md"), "/home/alice/notes/idea.md");
    }

    #[test]
    fn empty_is_home() {
        let s = user("alice", 1);
        assert_eq!(resolve_user_path(&s, ""), "/home/alice");
    }

    #[test]
    fn root_relative_stays_at_root() {
        let s = Session::root();
        assert_eq!(resolve_user_path(&s, "notes/idea.md"), "/notes/idea.md");
        assert_eq!(resolve_user_path(&s, ""), "/");
        assert_eq!(resolve_user_path(&s, "~"), "/");
    }

    #[test]
    fn delegate_home_wins() {
        let mut agent = user("claude-agent", 5);
        agent.delegate = Some(DelegateContext {
            uid: 1,
            gid: 1,
            groups: vec![1],
            username: "alice".to_string(),
        });
        assert_eq!(
            resolve_user_path(&agent, "notes/idea.md"),
            "/home/alice/notes/idea.md"
        );
    }
}
