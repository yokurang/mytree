# Command-Line Help for `rtree`

This document contains the help content for the `rtree` command-line program.

**Command Overview:**

* [`rtree`↴](#rtree)

## `rtree`

Rtree lets you view directory trees with optional hidden files, extension filtering, regex matching, and long-format metadata.

**Usage:** `rtree [OPTIONS] [PATH]`

###### **Arguments:**

* `<PATH>` — Root directory to start traversal

  Default value: `.`

###### **Options:**

* `-a`, `--all` — Include hidden files and directories

  Default value: `false`
* `-e`, `--extension <EXTENSIONS>` — Filter by file extensions (e.g. -e rs -e toml)
* `-r`, `--regex <REGEX>` — Filter entries by matching name with regex
* `-l`, `--long` — Enable long format output with size and timestamps

  Default value: `false`
* `-o`, `--output <OUTPUT>` — Write output to a file. Supports .gz compression
* `--pager` — Send output to pager (e.g. less)

  Default value: `false`



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
