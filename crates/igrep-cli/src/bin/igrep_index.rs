use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::time::Instant;

use anyhow::Result;
use clap::Parser;

use igrep_core::git_overlay;
use igrep_core::index::writer::IndexWriter;
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
    let mut writer = IndexWriter::new(config);
    let progress_start = Instant::now();
    let mut last_report = Instant::now();
    let mut bytes_total: u64 = 0;

    for (i, (doc_id, abs_path, rel_path)) in all_files.iter().enumerate() {
        let content = std::fs::read(abs_path).unwrap_or_default();
        bytes_total += content.len() as u64;
        let rel_str = rel_path.to_string_lossy();
        writer.add_file(*doc_id, &rel_str, &content);

        // Progress report: every 500ms or on the last file
        if last_report.elapsed().as_millis() >= 500 || i + 1 == total {
            let done = i + 1;
            let pct = (done as f64 / total as f64) * 100.0;
            let elapsed_s = progress_start.elapsed().as_secs_f64();
            let rate = if elapsed_s > 0.0 {
                done as f64 / elapsed_s
            } else {
                0.0
            };
            let mb = bytes_total as f64 / (1024.0 * 1024.0);
            eprint!(
                "\r\x1b[K  [{:>5.1}%] {}/{} files ({:.0} MB) \u{2014} {:.0} files/s",
                pct, done, total, mb, rate
            );
            last_report = Instant::now();
        }
        if args.verbose {
            eprintln!("  Indexed: {}", rel_str);
        }
    }
    eprintln!(); // newline after progress

    // Determine output directory (before finish() which consumes the writer)
    let output_dir = args
        .output_dir
        .unwrap_or_else(|| args.paths[0].join(".igrep"));
    std::fs::create_dir_all(&output_dir)?;

    // Sorting + writing can take a long time for large repos — show progress
    let entry_count = writer.entry_count();
    eprint!("  Sorting {} n-gram entries...", entry_count);
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
