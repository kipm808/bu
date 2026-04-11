# bu (Backup Utility)

`bu` is a lightweight, CLI-based versioning and backup tool written in Rust. It snapshots your current working directory into a sibling `bak/` folder, allowing you to track history, compare versions, and restore state without the overhead of a full VCS like Git.

```cpp
Usage: bu <command> [args]

Commands:
  save [-m MSG]       Backup current directory to ../bak/ (default)
  -m MSG...           Shorthand for save with multi-word message
  ls                  List backups and messages
  load [idx]          Restore backup (defaults to latest if idx omitted)
  find <pat> [file]   Search for pattern in historical files
  diff [i1] [i2] [-k] Diff latest vs current, or archive vs archive
                      (-k: keep extracted files in /tmp/)
  trim                Keep only latest backup and reset to 000
```

## Features

* **Sequential Snapshots**: Automatically creates tarballs with 3-digit indexing (`000`, `001`, etc.).
* **Smart Exclusions**: Automatically ignores build artifacts and heavy folders (e.g., `target/`, `.git/`, `__pycache__`, `objs/`).
* **In-Archive Search**: Search for text patterns across all historical backups without manual extraction.
* **Diffing Suite**:
    * Compare current directory vs. latest backup.
    * Compare specific historical versions against each other.
    * Optionally keep extracted files in `/tmp` for inspection using the `-k` flag.
* **Metadata Tracking**: Store and view custom notes for each backup.

## Installation

Ensure you have [Rust and Cargo](https://rustup.rs/) installed.

```bash
# Clone the repository
git clone [https://github.com/kipm808/bu.git]
cd bu

# Install the binary
cargo install --path .

