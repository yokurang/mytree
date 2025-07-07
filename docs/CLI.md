# Command-Line Help for `mytree`

This document contains the help content for the `mytree` command-line program.

**Command Overview:**

* [`mytree`↴](#mytree)

## `mytree`

You can use mytree to create custom visualizations of your project structure.The features supported are:1. Filtering results by file extensions2. Filtering results by regex matching3. Filtering results to include hidden files4. Enable long format output with file size and timestamps5. Sort results alphabetically (default)6. Sort results by file size7. Sort results by last updated timestamp8. Write results to a file as JSON
     

**Usage:** `mytree [OPTIONS] [PATH]`

###### **Arguments:**

* `<PATH>` — Root directory to start traversal

  Default value: `.`

###### **Options:**

* `-s`, `--sort <SORT_BY>` — Supply the argument with 'fs' to sort by file size, 'ts' to sort by last updated timestamp, or nothing to sort alphabetically (default)
* `-e`, `--extension <EXTENSION_FILTERS>` — Filter by file extensions (e.g. -e rs -e toml)
* `-a`, `--all` — Include hidden files and directories

  Default value: `false`
* `-r`, `--regex <REGEX>` — Filter entries by matching name with regex
* `-l`, `--long` — Enable long format output with file size and timestamps

  Default value: `false`
* `-j`, `--json <WRITE_JSON>` — Write directory tree in JSON format



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
