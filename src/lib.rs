use chrono::{DateTime, Local};
use clap::{arg, Parser};
use colored::*;
use regex::Regex;
use serde::Serialize;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Debug;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{fmt, fs};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Mytree is a terminal tool to visualize your project structure.",
    long_about = "You can use mytree to create custom visualizations of your project structure.\
     The features supported are:\
     1. Filtering results by file extensions\
     2. Filtering results by regex matching\
     3. Filtering results to include hidden files\
     4. Enable long format output with file size and timestamps\
     5. Sort results alphabetically (default)\
     6. Sort results by file size\
     7. Sort results by last updated timestamp\
     8. Write results to a file as JSON
     "
)]
pub struct Args {
    #[arg(default_value = ".", help = "Root directory to start traversal")]
    pub path: PathBuf,

    #[arg(
        short = 's',
        long = "sort",
        help = "Supply the argument with 'fs' to sort by file size, 'ts' to sort by last updated timestamp, or nothing to sort alphabetically (default)"
    )]
    pub sort_by: Option<String>,

    #[arg(
        short = 'e',
        long = "extension",
        help = "Filter by file extensions (e.g. -e rs -e toml)"
    )]
    pub extension_filters: Option<Vec<String>>,

    #[arg(
        short = 'a',
        long = "all",
        default_value_t = false,
        help = "Include hidden files and directories"
    )]
    pub show_hidden: bool,

    #[arg(
        short = 'r',
        long = "regex",
        help = "Filter entries by matching name with regex"
    )]
    pub regex: Option<String>,

    #[arg(
        short = 'l',
        long = "long",
        default_value_t = false,
        help = "Enable long format output with file size and timestamps"
    )]
    pub long_format: bool,

    #[arg(
        short = 'j',
        long = "json",
        help = "Write directory tree in JSON format"
    )]
    pub write_json: Option<String>,
}

struct PrintOptions {
    sort_by: SortBy,
    extension_filters: Option<HashSet<String>>,
    show_hidden: bool,
    regex_filter: Option<Regex>,
    long_format: bool,
    write_json: Option<String>,
}

struct Stats {
    dirs: usize,
    files: usize,
    size: u64,
}

struct EntryMeta {
    name: String,
    path: PathBuf,
    size: u64,
    mtime: SystemTime,
    is_dir: bool,
}

#[derive(Debug, Clone)]
enum SortBy {
    Alphabetical,
    FileSize,
    LastUpdatedTimestamp,
}

#[derive(Debug)]
pub struct ArgParseError {
    pub details: ArgParseErrorType,
}

#[derive(Debug)]
pub enum ArgParseErrorType {
    SortFlag(String),
    BadExtension(String),
    BadRegex(String),
}

impl fmt::Display for ArgParseErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgParseErrorType::SortFlag(flag) => write!(
                f,
                "invalid sort flag \"{flag}\" (expected \"fs\" or \"ts\")"
            ),
            ArgParseErrorType::BadExtension(ext) => write!(f, "invalid extension \"{ext}\""),
            ArgParseErrorType::BadRegex(msg) => write!(f, "invalid regex -> {msg}"),
        }
    }
}

impl fmt::Display for ArgParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "argument error -> {}", self.details)
    }
}

impl Error for ArgParseError {}

#[derive(Debug)]
pub struct TreeParseError {
    pub details: TreeParseType,
}

#[derive(Debug)]
pub enum TreeParseType {
    Io(String),
    InvalidInput(String),
}

impl fmt::Display for TreeParseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreeParseType::Io(msg) => write!(f, "IO error -> {msg}"),
            TreeParseType::InvalidInput(msg) => write!(f, "{msg}"),
        }
    }
}

impl fmt::Display for TreeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for TreeParseError {}

impl From<io::Error> for TreeParseError {
    fn from(e: io::Error) -> Self {
        TreeParseError {
            details: TreeParseType::Io(e.to_string()),
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    Args(ArgParseError),
    Tree(TreeParseError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Args(e) => Debug::fmt(&e, f),
            ParseError::Tree(e) => Debug::fmt(&e, f),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseError::Args(e) => Some(e),
            ParseError::Tree(e) => Some(e),
        }
    }
}

impl From<ArgParseError> for ParseError {
    fn from(e: ArgParseError) -> Self {
        Self::Args(e)
    }
}
impl From<TreeParseError> for ParseError {
    fn from(e: TreeParseError) -> Self {
        Self::Tree(e)
    }
}

impl From<ParseError> for io::Error {
    fn from(e: ParseError) -> io::Error {
        io::Error::new(ErrorKind::Other, e)
    }
}

#[derive(Debug, Serialize)]
struct TreeNode {
    name: String,
    path: PathBuf,
    size: u64,
    mtime: SystemTime,
    is_dir: bool,
    children: Option<Vec<TreeNode>>,
}

fn create_print_options_from_args(args: Args) -> Result<PrintOptions, ParseError> {
    let sort_by = match args.sort_by.as_deref() {
        Some("fs") => SortBy::FileSize,
        Some("ts") => SortBy::LastUpdatedTimestamp,
        Some(bad) => {
            return Err(ParseError::Args(ArgParseError {
                details: ArgParseErrorType::SortFlag(bad.into()),
            }));
        }
        None => SortBy::Alphabetical,
    };

    let extension_filters = if let Some(list) = args.extension_filters {
        let mut set = HashSet::with_capacity(list.len());
        for raw in list {
            let ext = raw.trim_start_matches('.');
            if ext.is_empty() {
                return Err(ParseError::Args(ArgParseError {
                    details: ArgParseErrorType::BadExtension(raw),
                }));
            }
            set.insert(ext.to_ascii_lowercase());
        }
        Some(set)
    } else {
        None
    };

    let regex_filter = if let Some(pattern) = args.regex {
        match Regex::new(&pattern) {
            Ok(re) => Some(re),
            Err(e) => {
                return Err(ParseError::Args(ArgParseError {
                    details: ArgParseErrorType::BadRegex(format!(
                        "invalid regex \"{pattern}\": {e}"
                    )),
                }));
            }
        }
    } else {
        None
    };

    Ok(PrintOptions {
        sort_by,
        extension_filters,
        show_hidden: args.show_hidden,
        regex_filter,
        long_format: args.long_format,
        write_json: args.write_json,
    })
}

/*
Return a vector of ordered row-level entries at a point in the directory
*/
fn create_ordered_row_level_entries(
    path: &Path,
    opts: &PrintOptions,
) -> Result<Vec<EntryMeta>, ParseError> {
    let iter = fs::read_dir(path).map_err(|e| {
        ParseError::Tree(TreeParseError {
            details: TreeParseType::Io(format!("error reading directory {}: {e}", path.display())),
        })
    })?;

    let mut meta_entries = Vec::new(); // allocate lazily

    for dir_entry in iter {
        let entry = dir_entry.map_err(|e| {
            ParseError::Tree(TreeParseError {
                details: TreeParseType::Io(format!(
                    "error reading an entry in {}: {e}",
                    path.display()
                )),
            })
        })?;

        let file_type = entry.file_type().map_err(|e| {
            ParseError::Tree(TreeParseError {
                details: TreeParseType::InvalidInput(format!(
                    "could not determine file type for {}: {e}",
                    entry.path().display()
                )),
            })
        })?;

        let name = entry.file_name().to_string_lossy().to_string();
        let ext = entry
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if !opts.show_hidden && name.starts_with('.') {
            continue;
        }
        if opts
            .extension_filters
            .as_ref()
            .map_or(false, |set| !set.contains(ext.as_str()))
        {
            continue;
        }
        if opts
            .regex_filter
            .as_ref()
            .map_or(false, |re| !re.is_match(&name))
        {
            continue;
        }

        let md = entry.metadata().map_err(|e| {
            ParseError::Tree(TreeParseError {
                details: TreeParseType::Io(format!(
                    "failed to read metadata for {}: {e}",
                    entry.path().display()
                )),
            })
        })?;

        meta_entries.push(EntryMeta {
            name,
            path: entry.path(),
            size: md.len(),
            mtime: md.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            is_dir: file_type.is_dir(),
        });
    }

    Ok(sort_meta_entries(meta_entries, &opts.sort_by))
}

fn sort_meta_entries(mut meta_entries: Vec<EntryMeta>, sort_criteria: &SortBy) -> Vec<EntryMeta> {
    match sort_criteria {
        SortBy::Alphabetical => {
            meta_entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }
        SortBy::FileSize => {
            meta_entries.sort_by(|a, b| a.size.cmp(&b.size));
        }
        SortBy::LastUpdatedTimestamp => {
            meta_entries.sort_by(|a, b| a.mtime.cmp(&b.mtime));
        }
    }
    meta_entries
}

/*
Return a vector of ordered row-level entries at a point in the directory
*/
fn build_directory_tree(root_path: &Path, opts: &PrintOptions) -> Result<TreeNode, ParseError> {
    let md = fs::metadata(root_path).map_err(|e| {
        ParseError::Tree(TreeParseError {
            details: TreeParseType::Io(format!(
                "failed to read metadata for {}: {e}",
                root_path.display()
            )),
        })
    })?;

    let entries = create_ordered_row_level_entries(root_path, opts)?;
    let mut kids = Vec::with_capacity(entries.len());
    for entry in entries {
        kids.push(build_tree_node_from_entry_meta(entry, opts)?);
    }

    Ok(TreeNode {
        name: root_path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| root_path.display().to_string()),
        path: root_path.to_owned(),
        size: md.len(),
        mtime: md.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        is_dir: true,
        children: Some(kids),
    })
}

fn build_tree_node_from_entry_meta(
    entry: EntryMeta,
    opts: &PrintOptions,
) -> Result<TreeNode, ParseError> {
    let children = if entry.is_dir {
        let subs = create_ordered_row_level_entries(&entry.path, opts)?;
        let mut nodes = Vec::with_capacity(subs.len());
        for sub in subs {
            nodes.push(build_tree_node_from_entry_meta(sub, opts)?);
        }
        Some(nodes)
    } else {
        None
    };

    Ok(TreeNode {
        name: entry.name,
        path: entry.path,
        size: entry.size,
        mtime: entry.mtime,
        is_dir: entry.is_dir,
        children,
    })
}

/*
Print the directory tree to standard out or write to JSON
*/
fn print_tree(
    node: &TreeNode,
    connector: &str,
    prefix_continuation: &str,
    stats: &mut Stats,
    opts: &PrintOptions,
    write_fn: &mut dyn FnMut(&str),
) {
    let line = format_entry_line(&node.path, &node.name, opts.long_format);
    write_fn(&format!("{}{}{}", prefix_continuation, connector, line));

    if node.is_dir {
        stats.dirs += 1;
    } else {
        stats.files += 1;
        stats.size += node.size;
    }

    if let Some(children) = node.children.as_ref() {
        let last_pos = children.len().saturating_sub(1);

        for (idx, child) in children.iter().enumerate() {
            let is_last = idx == last_pos;
            let child_conn = if is_last { "└── " } else { "├── " };
            let new_prefix = if is_last {
                format!("{}    ", prefix_continuation)
            } else {
                format!("{}│   ", prefix_continuation)
            };

            print_tree(child, child_conn, &new_prefix, stats, opts, write_fn);
        }
    }
}

fn print_ascii_tree(root: &TreeNode, opts: &PrintOptions, root_path: &Path) {
    let mut stats = Stats {
        dirs: 0,
        files: 0,
        size: 0,
    };

    println!("{}", root_path.display());

    let mut push_line = |line: &str| println!("{line}");

    if let Some(children) = root.children.as_ref() {
        let last = children.len().saturating_sub(1);
        for (idx, child) in children.iter().enumerate() {
            let is_last = idx == last;
            let connector = if is_last { "└── " } else { "├── " };
            let prefix = if is_last { "    " } else { "│   " };

            print_tree(child, connector, prefix, &mut stats, opts, &mut push_line);
        }
    }

    println!(
        "\n{} directories, {} files, {} bytes total",
        stats.dirs,
        stats.files,
        format_size(stats.size)
    );
}

fn format_entry_line(path: &Path, name: &str, long_format: bool) -> String {
    let is_hidden = name.starts_with('.') && name != "." && name != "..";
    let styled_name = if path.is_dir() {
        if is_hidden {
            name.blue().bold().dimmed().underline()
        } else {
            name.blue().bold()
        }
    } else if is_hidden {
        name.dimmed().underline()
    } else {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
        {
            Some(ext) if ext == "rs" => name.red().bold(),
            Some(ext) if ext == "py" => name.yellow().bold(),
            Some(ext) if ["c", "cpp", "h", "hpp"].contains(&ext.as_str()) => name.cyan().bold(),
            Some(ext) if ext == "cs" => name.magenta().bold(),
            Some(ext) if ext == "ml" || ext == "mli" => name.bright_green().bold(),
            Some(ext) if ext == "md" => name.white().italic(),
            Some(ext) if ext == "txt" => name.dimmed(),
            Some(ext) if ext == "json" => name.bright_yellow().bold(),
            _ => name.normal(),
        }
    };

    if long_format {
        match fs::metadata(path) {
            Ok(metadata) => {
                let size = format_size(metadata.len());
                let modified = metadata
                    .modified()
                    .ok()
                    .map(format_time)
                    .unwrap_or_else(|| "-".to_string());
                let created = metadata
                    .created()
                    .ok()
                    .map(format_time)
                    .unwrap_or_else(|| "-".to_string());
                format!(
                    "{}\n      {:<10} {:<12} {:<10} {:<20} {:<10} {:<20}",
                    styled_name, "Size:", size, "Modified:", modified, "Created:", created
                )
            }
            Err(e) => format!("{} (Error reading metadata: {})", styled_name, e),
        }
    } else {
        styled_name.to_string()
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut i = 0;
    while size >= 1024.0 && i < UNITS.len() - 1 {
        size /= 1024.0;
        i += 1;
    }
    format!("{:.1} {:<2}", size, UNITS[i])
}

fn format_time(system_time: SystemTime) -> String {
    let datetime: DateTime<Local> = system_time.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn write_tree_json<P>(nodes: &[TreeNode], dest: Option<P>) -> Result<(), ParseError>
where
    P: AsRef<Path>,
{
    let json_bytes = serde_json::to_vec_pretty(nodes).map_err(|e| {
        ParseError::Tree(TreeParseError {
            details: TreeParseType::InvalidInput(format!("serialising JSON: {e}")),
        })
    })?;

    let path: PathBuf = dest
        .map(|p| p.as_ref().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("file.json"));

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            ParseError::Tree(TreeParseError {
                details: TreeParseType::Io(format!("creating {:?}: {e}", parent)),
            })
        })?;
    }

    fs::write(&path, json_bytes).map_err(|e| {
        ParseError::Tree(TreeParseError {
            details: TreeParseType::Io(format!("writing {:?}: {e}", path)),
        })
    })
}

fn emit_json(tree: &TreeNode, dest_raw: &str) -> Result<(), ParseError> {
    let dest: Option<&Path> = if dest_raw.trim().is_empty() {
        None
    } else {
        Some(Path::new(dest_raw))
    };

    write_tree_json(std::slice::from_ref(tree), dest)?;

    println!(
        "Wrote directory tree to {}",
        dest.map(|p| p.display().to_string())
            .unwrap_or_else(|| "file.json".into())
    );

    Ok(())
}
pub fn run(args: Args) -> io::Result<()> {
    let path = &args.path.clone();
    let opts = create_print_options_from_args(args)?;
    let tree = build_directory_tree(path, &opts)?;

    if let Some(ref raw_dest) = opts.write_json {
        emit_json(&tree, raw_dest)?;
        return Ok(());
    }

    print_ascii_tree(&tree, &opts, path);
    Ok(())
}
