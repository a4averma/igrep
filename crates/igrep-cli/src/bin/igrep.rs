use std::collections::HashMap;
use std::path::PathBuf;
use std::process;

use anyhow::Result;
use clap::Parser;

use igrep_core::index::reader::IndexReader;
use igrep_core::search;

#[derive(Parser)]
#[command(name = "igrep", about = "Fast regex search with n-gram index")]
struct Args {
    /// Regex pattern to search for
    pattern: String,

    /// Directory to search (default: current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Path to index directory
    #[arg(long)]
    index_dir: Option<PathBuf>,

    /// Only print file paths
    #[arg(short = 'l', long)]
    files_with_matches: bool,

    /// Print match count per file
    #[arg(short, long)]
    count: bool,

    /// Print line numbers
    #[arg(short = 'n', long)]
    line_number: bool,

    /// Case-insensitive search
    #[arg(short, long)]
    ignore_case: bool,

    /// Only search files matching this regex
    #[arg(short = 'f', long)]
    file_filter: Option<String>,
}

fn main() {
    let args = Args::parse();
    match run(args) {
        Ok(found) => {
            if !found {
                process::exit(1); // No matches
            }
        }
        Err(e) => {
            eprintln!("igrep: error: {}", e);
            process::exit(2);
        }
    }
}

fn run(args: Args) -> Result<bool> {
    let index_dir = args.index_dir.unwrap_or_else(|| args.path.join(".igrep"));

    let reader = IndexReader::open(&index_dir)?;

    let file_filter_re = args
        .file_filter
        .as_ref()
        .map(|f| regex::Regex::new(f))
        .transpose()?;

    if args.files_with_matches {
        let files = search::search_files_with_options(
            &reader,
            &args.pattern,
            &args.path,
            args.ignore_case,
        )?;

        let files: Vec<_> = if let Some(ref re) = file_filter_re {
            files
                .into_iter()
                .filter(|f| re.is_match(&f.to_string_lossy()))
                .collect()
        } else {
            files
        };

        for f in &files {
            println!("{}", f.display());
        }
        return Ok(!files.is_empty());
    }

    let results =
        search::search_with_options(&reader, &args.pattern, &args.path, args.ignore_case)?;

    let results: Vec<_> = if let Some(ref re) = file_filter_re {
        results
            .into_iter()
            .filter(|r| re.is_match(&r.file_path.to_string_lossy()))
            .collect()
    } else {
        results
    };

    if args.count {
        let mut counts: HashMap<PathBuf, usize> = HashMap::new();
        for r in &results {
            *counts.entry(r.file_path.clone()).or_default() += 1;
        }
        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (path, count) in &sorted {
            println!("{}:{}", path.display(), count);
        }
        return Ok(!sorted.is_empty());
    }

    for r in &results {
        if args.line_number {
            println!(
                "{}:{}:{}",
                r.file_path.display(),
                r.line_number,
                r.line_content
            );
        } else {
            println!("{}:{}", r.file_path.display(), r.line_content);
        }
    }

    Ok(!results.is_empty())
}
