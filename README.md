# mytree

[`mytree`](https://crates.io/crates/mytree) is a small Rust CLI that prints a directory tree to the terminal.  
The output can be filtered (by extension or regex), include hidden items, show long-format metadata, be streamed through a pager, written to a file (optionally **gzip**-compressed), or emitted as machine-readable JSON.

---

## Installation

```bash
# needs Rust ≥1.70
cargo install mytree  # grabs the latest release from crates.io
```

> The binary will be installed to `$HOME/.cargo/bin`; make sure this directory is in your `PATH`.
> You can also find `mytree` on [crates.io](https://crates.io/crates/mytree).

---