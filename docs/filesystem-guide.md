# Filesystem Guide

This guide covers all filesystem operations in mdfs — creating files and directories, reading and writing content, moving and copying, searching, and using pipes.

## Key Concept: Markdown Only

mdfs only stores `.md` files. Every file you create must have the `.md` extension:

```
alice@markdownfs:/ $ touch notes.md     # works
alice@markdownfs:/ $ touch notes.txt    # error: only .md files are supported
```

Directories and symlinks are exempt from this rule.

## Working with Directories

### Creating Directories

```
alice@markdownfs:/ $ mkdir docs
alice@markdownfs:/ $ mkdir -p projects/web/frontend
```

The `-p` flag creates the entire directory tree (like `mkdir -p` on Unix). Without `-p`, the parent must already exist.

### Navigating

```
alice@markdownfs:/ $ cd docs
alice@markdownfs:/docs $ pwd
/docs

alice@markdownfs:/docs $ cd ..
alice@markdownfs:/ $ cd
alice@markdownfs:/ $
```

- `cd` with no arguments returns to `/`
- `cd ..` goes up one level
- Paths can be absolute (`/docs/notes`) or relative (`../other`)

### Listing Contents

```
alice@markdownfs:/docs $ ls
meeting-notes.md
readme.md
specs/

alice@markdownfs:/docs $ ls -l
-rw-r--r-- alice     alice          150 Apr 13 10:30 meeting-notes.md
-rw-r--r-- alice     alice           42 Apr 13 10:25 readme.md
drwxr-xr-x alice     alice            2 Apr 13 10:28 specs/
```

- `ls` — names only (directories have a trailing `/` indicator in tree view)
- `ls -l` — long format showing permissions, owner, group, size, date, and name
- `ls <path>` — list a specific directory

### Directory Tree

```
alice@markdownfs:/ $ tree
/
├── docs/
│   ├── meeting-notes.md
│   ├── readme.md
│   └── specs/
│       ├── api.md
│       └── design.md
└── notes/
    └── todo.md
```

`tree` recursively shows the full directory structure with Unicode box-drawing characters. Only entries you have permission to read are shown.

### Removing Directories

```
alice@markdownfs:/ $ rmdir empty-dir          # only works on empty directories
alice@markdownfs:/ $ rm -r docs/old-stuff      # recursive delete
```

## Working with Files

### Creating Files

`touch` creates an empty markdown file:

```
alice@markdownfs:/ $ touch notes.md
alice@markdownfs:/ $ touch docs/readme.md
```

If the file already exists, `touch` updates its modification time.

### Writing Content

**Inline write** — pass content directly:

```
alice@markdownfs:/ $ write notes.md # My Notes

This is the content of the file.
```

Everything after the filename becomes the file content (joined with spaces). The file is created if it doesn't exist.

**Pipe write** — write from another command's output:

```
alice@markdownfs:/ $ cat template.md | write notes.md
```

### Reading Files

```
alice@markdownfs:/ $ cat notes.md
# My Notes

This is the content of the file.
```

`cat` follows symlinks automatically. You can read multiple files:

```
alice@markdownfs:/ $ cat file1.md file2.md
```

### The `edit` Command

`edit` opens a multi-line editor for interactive content editing:

```
alice@markdownfs:/ $ edit notes.md
Current content:
    1 | # My Notes
    2 |
    3 | This is the content of the file.

Enter new content (type EOF to save, CANCEL to abort):
> # My Notes
> 
> Updated content with new information.
> 
> ## Section 2
> 
> More details here.
> EOF
Saved notes.md
[a1b2c3d4] edit notes.md
```

Key behaviors:
- Shows current content with line numbers
- Type your new content line by line
- `EOF` on its own line saves and auto-commits
- `CANCEL` on its own line aborts without saving

### Deleting Files

```
alice@markdownfs:/ $ rm notes.md
```

### Moving and Renaming

```
# Rename a file
alice@markdownfs:/ $ mv old-name.md new-name.md

# Move to a directory
alice@markdownfs:/ $ mv notes.md docs/

# Move and rename
alice@markdownfs:/ $ mv notes.md docs/meeting-notes.md
```

If the destination is an existing directory, the file is moved inside it keeping its original name.

### Copying Files

```
alice@markdownfs:/ $ cp original.md backup.md
alice@markdownfs:/ $ cp docs/readme.md archive/readme.md
```

The copy is owned by the user who performed the copy (not the original owner).

### File Metadata

```
alice@markdownfs:/ $ stat notes.md
  File: notes.md
  Size: 156
  Type: file
  Inode: 7
  Mode: 0644
  Uid: 1 (alice)
  Gid: 2 (alice)
  Created: 2025-04-13 10:30:00
  Modified: 2025-04-13 10:45:22
```

## Symbolic Links

mdfs supports symbolic links (hard links are not supported):

```
alice@markdownfs:/ $ ln -s docs/readme.md quick-link.md
alice@markdownfs:/ $ cat quick-link.md     # follows the symlink
alice@markdownfs:/ $ stat quick-link.md    # shows Type: symlink

alice@markdownfs:/ $ ls -l
lrwxrwxrwx alice     alice            14 Apr 13 11:00 quick-link.md -> docs/readme.md
```

Symlinks are followed transparently by `cat` and `write`. mdfs detects and prevents symlink loops.

## Searching

### grep — Search File Contents

Search for a regex pattern within files:

```
# Search a single file
alice@markdownfs:/ $ grep "TODO" notes.md
notes.md:5:TODO: finish the implementation

# Recursive search across all files
alice@markdownfs:/ $ grep -r "TODO" docs/
docs/readme.md:3:TODO: write introduction
docs/specs/api.md:12:TODO: document auth endpoints
docs/specs/design.md:8:TODO: add sequence diagram

# Search from root
alice@markdownfs:/ $ grep -r "bug" .
```

Output format: `file:line_number:matching_line`

### find — Find Files by Name

Search for files matching a glob pattern:

```
# Find all .md files
alice@markdownfs:/ $ find . -name "*.md"
./notes.md
./docs/readme.md
./docs/specs/api.md
./docs/specs/design.md

# Find files matching a specific pattern
alice@markdownfs:/ $ find docs -name "spec*"
docs/specs/api.md
docs/specs/design.md

# Find from current directory (default)
alice@markdownfs:/ $ find -name "readme*"
```

Supported glob characters:
- `*` — matches any sequence of characters
- `?` — matches a single character

## Pipes

mdfs supports Unix-style pipes to chain commands together. The output of one command becomes the input of the next.

### Available Pipe Commands

| Command | Behavior in Pipe |
|---|---|
| `cat` (no args) | Pass input through unchanged |
| `grep <pattern>` | Filter lines matching the regex pattern |
| `head [-n N]` | Keep only the first N lines (default: 10) |
| `tail [-n N]` | Keep only the last N lines (default: 10) |
| `wc [-l\|-w\|-c]` | Count lines (`-l`), words (`-w`), or bytes (`-c`). No flag = all three. |
| `write <file>` | Write pipe input to a file |

### Examples

**Filter and count:**

```
alice@markdownfs:/ $ grep -r "TODO" . | wc -l
7
```

**View the first few matches:**

```
alice@markdownfs:/ $ grep -r "error" logs/ | head -5
```

**Chain multiple filters:**

```
alice@markdownfs:/ $ cat report.md | grep "revenue" | tail -3
```

**Save filtered output to a file:**

```
alice@markdownfs:/ $ grep -r "TODO" . | write todos.md
```

**Read a file, filter, and count:**

```
alice@markdownfs:/ $ cat server-log.md | grep "ERROR" | head -10 | wc -l
```

### How Pipes Work

1. The first command runs normally, producing output text
2. Each subsequent command receives the previous output as input
3. Pipe-aware commands (`grep`, `head`, `tail`, `wc`, `cat`, `write`) process the input accordingly
4. If any command in the pipeline produces empty output, the pipeline stops

## Practical Walkthrough

Here's a complete session showing typical filesystem workflows:

```
# Create a project structure
alice@markdownfs:/ $ mkdir -p myproject/docs
alice@markdownfs:/ $ mkdir -p myproject/notes

# Create some files
alice@markdownfs:/ $ touch myproject/docs/readme.md
alice@markdownfs:/ $ touch myproject/docs/api.md
alice@markdownfs:/ $ touch myproject/notes/ideas.md

# Write content
alice@markdownfs:/ $ write myproject/docs/readme.md # My Project

A markdown-first documentation system.

## Getting Started

Install and run.
alice@markdownfs:/ $ write myproject/docs/api.md # API Reference

TODO: document all endpoints

## Authentication

TODO: describe auth flow
alice@markdownfs:/ $ write myproject/notes/ideas.md # Ideas

- Build a search feature
- Add TODO tracking
- Performance improvements

# View the structure
alice@markdownfs:/ $ tree myproject
myproject/
├── docs/
│   ├── api.md
│   └── readme.md
└── notes/
    └── ideas.md

# Search across the project
alice@markdownfs:/ $ grep -r "TODO" myproject/
myproject/docs/api.md:3:TODO: document all endpoints
myproject/docs/api.md:7:TODO: describe auth flow

# Count TODOs
alice@markdownfs:/ $ grep -r "TODO" myproject/ | wc -l
2

# Copy a file as a backup
alice@markdownfs:/ $ cp myproject/docs/readme.md myproject/docs/readme-backup.md

# Rename a file
alice@markdownfs:/ $ mv myproject/notes/ideas.md myproject/notes/backlog.md

# Clean up
alice@markdownfs:/ $ rm myproject/docs/readme-backup.md
```
