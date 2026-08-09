[![Latest Release](https://img.shields.io/github/v/release/kipm808/bu?display_name=tag&sort=semver)](https://github.com/kipm808/bu/releases)

# bu (Backup Utility)

`bu` is a lightweight, CLI-based versioning and backup tool written in Rust. It snapshots your current working directory into a sibling `bak/` folder, allowing you to track history, compare versions, and restore state without the overhead of a full VCS like Git.

```cpp
Usage: bu <command> [args]

Commands:
  save [-m MSG]       Backup current directory to ../bak/ (default)
  -m MSG...           Shorthand for save with multi-word message
  ls, l, -l           List backups and messages
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

## Configuration

`bu` looks for an optional exclusion file at `~/.burc/excludes`. If this file exists, `bu` will use the patterns defined within it (one per line). Lines starting with `#` are treated as comments.

### Custom Excludes
To create a custom exclusion list:
```bash
mkdir -p ~/.burc
echo "[filename]" >> ~/.burc/excludes

Default Excludes

If no configuration file is found, bu defaults to excluding the following common build and metadata directories:
    bak/
    tmp/
    objs/
    build/
    .git/
    target/
    __pycache__/
    Any file with the .sif extension.
```

## Installation

### Option 1: Download Pre-built Binaries

Click the release badge at the top of this README to access the latest release, or visit [Releases](https://github.com/kipm808/bu/releases) directly.

Download the appropriate binary for your platform:
- Windows: `bu-windows.exe`
- Linux: `bu-linux`
- macOS: `bu-macos`

#### Linux and macOS Setup

After downloading, make the binary executable:

```bash
chmod +x bu-linux   # or bu-macos
```

On macOS, you may need to allow the binary to run. If you see a security warning, either:
- Right-click the file, select "Open", and confirm
- Or run: `xattr -d com.apple.quarantine bu-macos`

Move the binary to a location in your PATH:

```bash
sudo mv bu-linux /usr/local/bin/bu   # Linux
sudo mv bu-macos /usr/local/bin/bu   # macOS
```

### Option 2: Build from Source

Ensure you have [Rust and Cargo](https://rustup.rs/) installed.

```bash
# Clone the repository
git clone https://github.com/kipm808/bu.git
cd bu

# Install the binary
cargo install --path .

