Here is a clean, concise `README.md` with no emojis or icons:

---

````markdown
# rtree

**rtree** is a command-line utility for visualizing directory structures, similar to the Unix `tree` command, with additional features like extension filtering, hidden file support, long-format output, and optional output redirection.

## Features

- Recursive directory traversal with optional maximum depth
- Option to include or exclude hidden files
- Filter output by file extensions
- Long-format display showing file size, creation, and modification timestamps
- Colored output grouped by file type
- Support for writing output to a file, including gzip-compressed `.gz` files
- Option to pipe output through a pager such as `less`

## Usage

```bash
rtree [OPTIONS]
````

### Examples

```bash
rtree                          # Display full tree from current directory
rtree -a                       # Include hidden files
rtree -d 2                     # Traverse only two levels deep
rtree -e rs -e md              # Show only files with .rs and .md extensions
rtree -l                       # Show file sizes and timestamps
rtree -o tree.txt              # Write output to a plain text file
rtree --pager                  # Pipe output to pager (e.g., less)
```

## Installation

Build and install locally with Cargo:

```bash
cargo install --path .
```

Or clone the repository and build:

```bash
git clone https://github.com/yourusername/rtree
cd rtree
cargo build --release
```

## Documentation

A full command reference is generated at `docs/CLI.md`:

```bash
cargo run --bin gendoc
```

## License

This project is licensed under the MIT License.
---
