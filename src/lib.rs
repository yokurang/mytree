// src/lib.rs
use chrono::{DateTime, Local};
use clap::Parser;
use colored::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use regex::Regex;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Rtree is a terminal tool to visualize your folder structure.",
    long_about = "Rtree lets you view directory trees with optional hidden files, extension filtering, regex matching, and long-format metadata."
)]
pub struct Args {
    #[arg(default_value = ".", help = "Root directory to start traversal")]
    pub path: PathBuf,

    #[arg(
        short = 'a',
        long = "all",
        default_value_t = false,
        help = "Include hidden files and directories"
    )]
    pub all: bool,

    #[arg(
        short = 'e',
        long = "extension",
        help = "Filter by file extensions (e.g. -e rs -e toml)"
    )]
    pub extensions: Vec<String>,

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
        help = "Enable long format output with size and timestamps"
    )]
    pub long: bool,

    #[arg(
        short = 'o',
        long = "output",
        help = "Write output to a file. Supports .gz compression"
    )]
    pub output: Option<PathBuf>,

    #[arg(
        long = "pager",
        default_value_t = false,
        help = "Send output to pager (e.g. less)"
    )]
    pub pager: bool,
}

struct Stats {
    dirs: usize,
    files: usize,
}

struct EntryMeta {
    entry: fs::DirEntry,
    type_priority: u8,
    ext: String,
    name: String,
}

pub fn run(args: Args) -> io::Result<()> {
    let extension_set: HashSet<String> = args.extensions.into_iter().collect();
    let regex_filter = match &args.regex {
        Some(pattern) => {
            Some(Regex::new(pattern).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?)
        }
        None => None,
    };
    let mut stats = Stats { dirs: 0, files: 0 };

    let use_pager = args.pager;
    let mut output_buffer = Vec::new();

    let root_display = format!("{}\n", args.path.display());
    output_buffer.extend_from_slice(root_display.as_bytes());

    {
        let mut write_fn = |line: &str| {
            output_buffer.extend_from_slice(line.as_bytes());
            output_buffer.push(b'\n');
        };

        let opts = PrintOptions {
            show_all: args.all,
            extension_filters: &extension_set,
            regex_filter: regex_filter.as_ref(),
            long_format: args.long,
        };

        print_entries_to_buffer(&args.path, "", &mut stats, &opts, &mut write_fn)?;

        let summary = format!("\n{} directories, {} files", stats.dirs, stats.files);
        output_buffer.extend_from_slice(summary.as_bytes());
        output_buffer.push(b'\n');
    }

    if let Some(path) = args.output {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext == "gz" {
                let file = fs::File::create(&path)?;
                let mut encoder = GzEncoder::new(file, Compression::default());
                encoder.write_all(&output_buffer)?;
                encoder.finish()?;
            } else {
                fs::write(path, output_buffer)?;
            }
        } else {
            fs::write(path, output_buffer)?;
        }
    } else if use_pager {
        let mut pager = Command::new("less")
            .arg("-R")
            .stdin(Stdio::piped())
            .spawn()?;
        if let Some(stdin) = pager.stdin.as_mut() {
            stdin.write_all(&output_buffer)?;
        }
        pager.wait()?;
    } else {
        io::stdout().write_all(&output_buffer)?;
    }

    Ok(())
}

struct PrintOptions<'a> {
    show_all: bool,
    extension_filters: &'a HashSet<String>,
    regex_filter: Option<&'a Regex>,
    long_format: bool,
}

fn print_entries_to_buffer(
    root_path: &Path,
    root_prefix: &str,
    stats: &mut Stats,
    opts: &PrintOptions,
    write_fn: &mut dyn FnMut(&str),
) -> io::Result<()> {
    use std::collections::VecDeque;

    let mut stack = VecDeque::new();
    stack.push_back((root_path.to_path_buf(), root_prefix.to_string()));

    while let Some((path, prefix)) = stack.pop_back() {
        let entries = read_filtered_entries(
            &path,
            opts.show_all,
            opts.extension_filters,
            opts.regex_filter,
        )?;
        let total = entries.len();

        for (i, entry) in entries.into_iter().enumerate() {
            let child_path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_last = i == total - 1;
            let connector = if is_last { "`--" } else { "|--" };
            let line = format_entry_line(&child_path, &name, opts.long_format);
            write_fn(&format!("{}{}{}", prefix, connector, line));

            if child_path.is_dir() {
                stats.dirs += 1;
                let new_prefix = if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}│   ", prefix)
                };
                stack.push_back((child_path, new_prefix));
            } else {
                stats.files += 1;
            }
        }
    }

    Ok(())
}

fn format_entry_line(path: &Path, name: &str, long_format: bool) -> String {
    let is_hidden = name.starts_with('.');
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

    let info = if long_format {
        match fs::metadata(path) {
            Ok(metadata) => {
                let size = format_size(metadata.len());
                let modified = metadata
                    .modified()
                    .ok()
                    .map(format_time)
                    .unwrap_or("-".to_string());
                let created = metadata
                    .created()
                    .ok()
                    .map(format_time)
                    .unwrap_or("-".to_string());
                format!(" {:>10}  {}  {}", size, modified, created)
            }
            Err(e) => format!("Error: {}", e),
        }
    } else {
        String::new()
    };

    format!("{}{}", styled_name, info)
}

fn read_filtered_entries(
    path: &Path,
    show_all: bool,
    extension_filters: &HashSet<String>,
    regex_filter: Option<&Regex>,
) -> io::Result<Vec<fs::DirEntry>> {
    let mut entries_meta = vec![];

    for entry_result in fs::read_dir(path)? {
        let entry = entry_result?;
        let file_name_os = entry.file_name();

        // Safely handle non-UTF8 filenames
        let name = match file_name_os.to_str() {
            Some(n) => n,
            None => continue, // skip non-UTF8 entries
        };

        if !show_all && name.starts_with('.') {
            continue;
        }

        if !extension_filters.is_empty() {
            let ext_match = entry
                .path()
                .extension()
                .and_then(OsStr::to_str)
                .map(|e| extension_filters.contains(e))
                .unwrap_or(false);
            if !ext_match {
                continue;
            }
        }

        if let Some(re) = regex_filter {
            if !re.is_match(name) {
                continue;
            }
        }

        let ext = entry
            .path()
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_lowercase();

        let file_type = entry.file_type()?;
        let type_priority = if file_type.is_dir() {
            0
        } else if file_type.is_file() {
            1
        } else if file_type.is_symlink() {
            2
        } else {
            3
        };

        entries_meta.push(EntryMeta {
            entry,
            type_priority,
            ext,
            name: name.to_string(),
        });
    }

    entries_meta.sort_by(|a, b| {
        (a.type_priority, &a.ext, &a.name).cmp(&(b.type_priority, &b.ext, &b.name))
    });

    Ok(entries_meta.into_iter().map(|e| e.entry).collect())
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

// === TESTS ===

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0.0 B ");
        assert_eq!(format_size(1023), "1023.0 B ");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_format_time() {
        let now = SystemTime::now();
        let formatted = format_time(now);
        assert!(formatted.contains('-'));
    }

    #[test]
    fn test_hidden_file_filtering() -> io::Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join(".hidden"))?;
        File::create(dir.path().join("visible.txt"))?;

        let no_hidden = read_filtered_entries(dir.path(), false, &HashSet::new(), None)?;
        let names: Vec<_> = no_hidden
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["visible.txt"]);

        let all_files = read_filtered_entries(dir.path(), true, &HashSet::new(), None)?;
        let names: Vec<_> = all_files
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&".hidden".to_string()));
        assert!(names.contains(&"visible.txt".to_string()));
        Ok(())
    }

    #[test]
    fn test_extension_filtering() -> io::Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("main.rs"))?;
        File::create(dir.path().join("README.md"))?;
        File::create(dir.path().join("LICENSE"))?;

        let mut filters = HashSet::new();
        filters.insert("rs".to_string());
        filters.insert("md".to_string());

        let entries = read_filtered_entries(dir.path(), true, &filters, None)?;
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        assert!(names.contains(&"main.rs".to_string()));
        assert!(names.contains(&"README.md".to_string()));
        assert!(!names.contains(&"LICENSE".to_string()));
        Ok(())
    }

    #[test]
    fn test_regex_filtering() -> io::Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("data1.csv"))?;
        File::create(dir.path().join("data2.csv"))?;
        File::create(dir.path().join("notes.txt"))?;

        let regex = Regex::new(r"^data.*").unwrap();
        let entries = read_filtered_entries(dir.path(), true, &HashSet::new(), Some(&regex))?;
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        assert!(names.contains(&"data1.csv".to_string()));
        assert!(names.contains(&"data2.csv".to_string()));
        assert!(!names.contains(&"notes.txt".to_string()));
        Ok(())
    }

    #[test]
    fn test_format_entry_line_outputs_metadata() -> io::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("info.txt");
        File::create(&file_path)?;
        let line = format_entry_line(&file_path, "info.txt", true);
        assert!(line.contains("info.txt"));
        assert!(line.contains("B") || line.contains("KB"));
        Ok(())
    }
}
