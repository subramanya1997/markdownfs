# User Management

mdfs has a built-in multi-user system with Unix-style permissions. This guide covers creating users, managing groups, setting permissions, and controlling access.

## Default State

A fresh mdfs instance has one user and two groups:

| Entity | ID | Notes |
|---|---|---|
| `root` user | uid=0 | Superuser, bypasses all permission checks |
| `root` group | gid=0 | Root's primary group |
| `wheel` group | gid=1 | Admin group — members can create users/groups |

On first CLI launch, you create an admin user who is automatically added to the `wheel` group.

## Creating Users

### Regular Users

Any admin (root or `wheel` member) can create users:

```
alice@markdownfs:~ $ adduser bob
User 'bob' created (uid=2)
Home directory: /home/bob
```

Each new user automatically gets:
- A unique uid (auto-incremented)
- A **personal group** with the same name (e.g., group `bob` with a unique gid)
- A **home directory** at `/home/<name>` owned by the user
- Default permissions to read public files and create files in directories they own

### Agent Users

Agents are special users designed for programmatic access (scripts, bots, CI):

```
alice@markdownfs:/ $ addagent deploy-bot
Created agent: deploy-bot (uid=3)
Token: a1b2c3d4e5f6...  (save this — shown only once)
```

The token is used for HTTP API authentication via `Authorization: Bearer <token>`. The raw token is shown once — mdfs only stores its SHA-256 hash internally.

### Deleting Users

Only `root` can delete users:

```
root@markdownfs:/ $ su root
root@markdownfs:/ $ deluser bob
Deleted user: bob
```

Root cannot be deleted.

## Managing Groups

### Creating Groups

```
alice@markdownfs:/ $ addgroup engineering
Created group: engineering (gid=3)
```

### Adding Users to Groups

```
alice@markdownfs:/ $ usermod -aG engineering bob
Added bob to engineering
```

### Removing Users from Groups

```
alice@markdownfs:/ $ usermod -rG engineering bob
Removed bob from engineering
```

### Deleting Groups

Only root can delete groups. The `root` and `wheel` groups are protected:

```
root@markdownfs:/ $ delgroup engineering
Deleted group: engineering
```

### Viewing Group Memberships

```
alice@markdownfs:/ $ groups
alice wheel

alice@markdownfs:/ $ groups bob
bob engineering
```

### Viewing User Identity

```
alice@markdownfs:/ $ whoami
alice

alice@markdownfs:/ $ id
uid=1(alice) gid=2(alice) groups=2(alice),1(wheel)

alice@markdownfs:/ $ id bob
uid=2(bob) gid=3(bob) groups=3(bob),4(engineering)
```

## Switching Users

Admins (root or wheel members) can switch to any user:

```
alice@markdownfs:/ $ su bob
bob@markdownfs:/ $ whoami
bob
```

## Permission Model

mdfs uses standard Unix-style permission bits: **owner**, **group**, and **other**, each with read (r), write (w), and execute (x).

### Permission Bits

| Bit | Octal | File | Directory |
|---|---|---|---|
| Read (r) | 4 | View content (`cat`) | List entries (`ls`) |
| Write (w) | 2 | Modify content (`write`) | Add/remove entries (`touch`, `rm`, `mv`) |
| Execute (x) | 1 | *(unused for files)* | Traverse (`cd`, path resolution) |

### Default Permissions

| Type | Mode | Human-readable |
|---|---|---|
| New file | `0644` | `rw-r--r--` — owner reads/writes, everyone else reads |
| New directory | `0755` | `rwxr-xr-x` — owner has full access, everyone else can list and traverse |
| New symlink | `0777` | `rwxrwxrwx` — unrestricted (target permissions still apply) |

### Reading Permissions in `ls -l`

```
alice@markdownfs:/ $ ls -l
drwxr-xr-x alice     engineering      3 Apr 13 10:30 project/
-rw-r--r-- alice     alice           42 Apr 13 10:31 readme.md
lrwxrwxrwx alice     alice            9 Apr 13 10:32 link.md -> readme.md
```

The first column reads as: `[type][owner rwx][group rwx][other rwx]`

- `d` = directory, `l` = symlink, `-` = file
- `rwx` = read, write, execute enabled
- `-` = that bit is not set

### Changing Permissions with `chmod`

Use octal notation. Only the file owner or root can change permissions:

```
alice@markdownfs:/ $ chmod 700 private/
alice@markdownfs:/ $ chmod 640 secret.md
```

Common permission patterns:

| Mode | Meaning | Use Case |
|---|---|---|
| `755` | `rwxr-xr-x` | Shared directory — everyone can read and traverse |
| `700` | `rwx------` | Private directory — owner only |
| `644` | `rw-r--r--` | Shared file — everyone can read |
| `600` | `rw-------` | Private file — owner only |
| `664` | `rw-rw-r--` | Group-writable file |
| `775` | `rwxrwxr-x` | Group-writable directory |

### Changing Ownership with `chown`

```
# Change owner (root only)
root@markdownfs:/ $ chown bob readme.md

# Change owner and group
root@markdownfs:/ $ chown bob:engineering readme.md

# Change group only (owner can do this if they're in the target group)
alice@markdownfs:/ $ chown alice:engineering project/
```

Rules:
- Changing the **owner** requires root
- Changing the **group** requires root, OR the current owner must be a member of the target group

### Viewing File Metadata with `stat`

```
alice@markdownfs:/ $ stat readme.md
  File: readme.md
  Size: 42
  Type: file
  Inode: 5
  Mode: 0644
  Uid: 1 (alice)
  Gid: 2 (alice)
  Created: 2025-04-13 10:30:00
  Modified: 2025-04-13 10:31:15
```

## Special Permission Bits

### Sticky Bit (`1xxx`)

When set on a directory, only the file owner, directory owner, or root can delete or rename files within it. Useful for shared directories:

```
alice@markdownfs:/ $ mkdir shared
alice@markdownfs:/ $ chmod 1777 shared
```

Now anyone can create files in `shared/`, but users can only delete their own files.

### Setgid Bit (`2xxx`)

When set on a directory, new files and subdirectories inherit the directory's group instead of the creator's primary group. Subdirectories also inherit the setgid bit:

```
alice@markdownfs:/ $ mkdir team-docs
alice@markdownfs:/ $ chown alice:engineering team-docs
alice@markdownfs:/ $ chmod 2775 team-docs

# Bob creates a file — it gets group "engineering" automatically
bob@markdownfs:/ $ touch team-docs/notes.md
bob@markdownfs:/ $ ls -l team-docs/
-rw-r--r-- bob       engineering      0 Apr 13 11:00 notes.md
```

## Delegation (Agent Impersonation)

Agents can act on behalf of users using the delegation system. When delegated, all operations use the **intersection** of the agent's and the delegated user's permissions (least-privilege):

```
deploy-bot@markdownfs:/ $ delegate bob
Delegating as bob

deploy-bot@markdownfs:/ $ touch docs/deploy-notes.md
# File is owned by bob, and both deploy-bot AND bob must have permission

deploy-bot@markdownfs:/ $ undelegate
Delegation cleared
```

You can also delegate for a group:

```
deploy-bot@markdownfs:/ $ delegate :engineering
```

## Permission Requirements by Operation

| Operation | Required Permission |
|---|---|
| `ls`, `tree` | Read + Execute on directory |
| `cd` | Execute on target directory |
| `cat` | Read on file |
| `touch` (new file) | Write + Execute on parent directory |
| `touch` (existing) | Write on file |
| `write` (new file) | Write on parent directory |
| `write` (existing) | Write on file |
| `mkdir` | Write + Execute on parent directory |
| `rm` | Write + Execute on parent directory |
| `mv` | Write + Execute on both source and destination parents |
| `cp` | Read on source, Write + Execute on destination parent |
| `chmod` | Must be owner or root |
| `chown` (uid) | Root only |
| `ln -s` | Write + Execute on link's parent directory |
| `grep`, `find` | Read on file/directory |
| Path traversal | Execute on every intermediate directory in the path |

## Practical Example: Team Setup

Here's a complete walkthrough setting up a team workspace. Since top-level directories are owned by root, we switch to root to create the shared structure:

```
# Start as admin
alice@markdownfs:~ $

# Create team members (each gets a home directory automatically)
alice@markdownfs:~ $ adduser bob
User 'bob' created (uid=2)
Home directory: /home/bob

alice@markdownfs:~ $ adduser carol
alice@markdownfs:~ $ addagent ci-bot

# Create a shared team group
alice@markdownfs:~ $ addgroup dev-team
alice@markdownfs:~ $ usermod -aG dev-team alice
alice@markdownfs:~ $ usermod -aG dev-team bob
alice@markdownfs:~ $ usermod -aG dev-team carol

# Switch to root to create shared top-level directories
alice@markdownfs:~ $ su root

# Create a shared workspace with setgid
root@markdownfs:~ $ mkdir /project
root@markdownfs:~ $ chown alice:dev-team /project
root@markdownfs:~ $ chmod 2775 /project

# Create a drop-box directory (sticky bit — anyone can add, only owners can delete)
root@markdownfs:~ $ mkdir /submissions
root@markdownfs:~ $ chmod 1777 /submissions

# Switch back to alice
root@markdownfs:~ $ su alice

# Alice can also create private dirs in her home
alice@markdownfs:~ $ mkdir private
alice@markdownfs:~ $ chmod 700 private

# Bob creates a file in the shared project — inherits dev-team group
alice@markdownfs:~ $ su bob
bob@markdownfs:~ $ touch /project/design.md
bob@markdownfs:~ $ write /project/design.md # System Design Draft
bob@markdownfs:~ $ ls -l /project/
-rw-r--r-- bob       dev-team        22 Apr 13 11:05 design.md
```
