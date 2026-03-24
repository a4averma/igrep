use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use anyhow::Result;
use clap::Parser;
use rayon::prelude::*;

use igrep_core::git_overlay;
use igrep_core::index::writer::{merge_maps, process_file_to_map, IndexWriter};
use igrep_core::types::IndexConfig;
use igrep_core::walker::walk;

#[derive(Parser)]
#[command(name = "igrep-index", about = "Build trigram search index for igrep")]
struct Args {
    /// Directories or files to index
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Output directory for index files
    #[arg(short, long)]
    output_dir: Option<PathBuf>,

    /// Skip files larger than this size in bytes
    #[arg(long, default_value = "52428800")]
    max_file_size: u64,

    /// Print progress
    #[arg(short, long)]
    verbose: bool,
}

fn run(args: Args) -> Result<()> {
    let start = Instant::now();

    let config = IndexConfig {
        max_file_size: args.max_file_size,
        ..IndexConfig::default()
    };

    // Walk all paths and collect files
    let mut all_files: Vec<(u32, PathBuf, PathBuf)> = Vec::new(); // (doc_id, absolute, relative)
    let mut next_doc_id: u32 = 0;

    for root in &args.paths {
        let root = root.canonicalize().unwrap_or_else(|_| root.clone());
        let files = walk(&root, &config)?;
        for (_orig_id, abs_path) in files {
            let rel_path = abs_path
                .strip_prefix(&root)
                .unwrap_or(&abs_path)
                .to_path_buf();
            all_files.push((next_doc_id, abs_path, rel_path));
            next_doc_id += 1;
        }
    }

    if args.verbose {
        eprintln!("Found {} files to index", all_files.len());
    }

    // Build index
    let total = all_files.len();
    let progress_start = Instant::now();
    let files_done = Arc::new(AtomicUsize::new(0));
    let bytes_done = Arc::new(AtomicU64::new(0));

    let progress_files = Arc::clone(&files_done);
    let progress_bytes = Arc::clone(&bytes_done);
    let progress_handle = thread::spawn(move || loop {
        thread::sleep(std::time::Duration::from_millis(500));
        let done = progress_files.load(Ordering::Relaxed);
        let bytes = progress_bytes.load(Ordering::Relaxed);
        let pct = if total > 0 {
            (done as f64 / total as f64) * 100.0
        } else {
            100.0
        };
        let elapsed_s = progress_start.elapsed().as_secs_f64();
        let rate = if elapsed_s > 0.0 {
            done as f64 / elapsed_s
        } else {
            0.0
        };
        let mb = bytes as f64 / (1024.0 * 1024.0);
        eprint!(
            "\r\x1b[K  [{:>5.1}%] {}/{} files ({:.0} MB) \u{2014} {:.0} files/s",
            pct, done, total, mb, rate
        );
        std::io::stderr().flush().ok();
        if done >= total {
            break;
        }
    });

    let (merged_map, all_paths): (HashMap<u32, Vec<u32>>, Vec<(u32, String)>) = all_files
        .par_iter()
        .fold(
            || (HashMap::new(), Vec::new()),
            |mut acc, (doc_id, abs_path, rel_path)| {
                let content = std::fs::read(abs_path).unwrap_or_default();
                bytes_done.fetch_add(content.len() as u64, Ordering::Relaxed);
                let file_map = process_file_to_map(&content, *doc_id, &config);
                merge_maps(&mut acc.0, file_map);
                acc.1
                    .push((*doc_id, rel_path.to_string_lossy().to_string()));
                files_done.fetch_add(1, Ordering::Relaxed);
                acc
            },
        )
        .reduce(
            || (HashMap::new(), Vec::new()),
            |mut left, mut right| {
                merge_maps(&mut left.0, right.0);
                left.1.append(&mut right.1);
                left
            },
        );

    progress_handle.join().ok();
    eprintln!(); // newline after progress

    let mut writer = IndexWriter::new(config);
    writer.add_map_bulk(merged_map, all_paths);

    // Determine output directory (before finish() which consumes the writer)
    let output_dir = args
        .output_dir
        .unwrap_or_else(|| args.paths[0].join(".igrep"));
    std::fs::create_dir_all(&output_dir)?;

    // Encoding + writing can take a long time for large repos — show progress
    let entry_count = writer.entry_count();
    eprint!("  Encoding {} n-gram entries...", entry_count);
    std::io::stderr().flush().ok();
    let meta = writer.finish(&output_dir)?;
    eprintln!(" done. ({} unique n-grams)", meta.ngram_count);

    // Write commit SHA if in a git repo
    if let Ok(sha) = git_overlay::current_head_sha(&args.paths[0]) {
        let _ = git_overlay::write_commit_sha(&output_dir, &sha);
    }

    let elapsed = start.elapsed();
    println!(
        "Indexed {} files ({} n-grams) in {:.1}s. Index written to {}",
        meta.file_count,
        meta.ngram_count,
        elapsed.as_secs_f64(),
        output_dir.display()
    );

    Ok(())
}

fn main() {
    let args = Args::parse();
    if let Err(e) = run(args) {
        eprintln!("error: {:#}", e);
        process::exit(1);
    }
}
