use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use igrep_core::index::writer::{
    merge_maps, process_file_ngrams, process_file_to_map, IndexWriter,
};
use igrep_core::types::IndexConfig;

fn synthetic_content(size: usize, seed: usize) -> Vec<u8> {
    (0..size).map(|i| ((i + seed) % 96 + 32) as u8).collect()
}

fn bench_index_end_to_end(c: &mut Criterion) {
    let config = IndexConfig::default();
    let num_files: u32 = 10_000;
    let file_size = 1024;

    // Pre-build the aggregated map using the new HashMap approach
    let mut all_map: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut file_paths = Vec::new();
    for doc_id in 0..num_files {
        let content = synthetic_content(file_size, doc_id as usize);
        let file_map = process_file_to_map(&content, doc_id, &config);
        merge_maps(&mut all_map, file_map);
        file_paths.push((doc_id, format!("file_{}.txt", doc_id)));
    }

    c.bench_function("index_end_to_end_10k_files", |b| {
        b.iter(|| {
            let mut writer = IndexWriter::new(config.clone());
            writer.add_map_bulk(black_box(all_map.clone()), black_box(file_paths.clone()));
            let out_dir = tempfile::tempdir().unwrap();
            let meta = writer.finish(out_dir.path()).unwrap();
            black_box(meta);
        });
    });
}

fn bench_hash_aggregation(c: &mut Criterion) {
    let config = IndexConfig::default();
    let num_files: u32 = 10_000;
    let file_size = 1024;

    // Pre-generate file contents
    let files: Vec<Vec<u8>> = (0..num_files)
        .map(|i| synthetic_content(file_size, i as usize))
        .collect();

    c.bench_function("hash_aggregation_10k_files", |b| {
        b.iter(|| {
            let mut map = HashMap::new();
            for (doc_id, content) in files.iter().enumerate() {
                let file_map = process_file_to_map(black_box(content), doc_id as u32, &config);
                merge_maps(&mut map, file_map);
            }
            let mut keys: Vec<u32> = map.keys().copied().collect();
            keys.sort_unstable();
            black_box((map, keys));
        });
    });
}

fn bench_process_file_ngrams(c: &mut Criterion) {
    let config = IndexConfig::default();
    let sizes: &[usize] = &[1024, 10_240, 102_400];

    let mut group = c.benchmark_group("process_file_ngrams");
    for &size in sizes {
        let content = synthetic_content(size, 0);
        group.bench_with_input(BenchmarkId::from_parameter(size), &content, |b, inp| {
            b.iter(|| process_file_ngrams(black_box(inp), 0, &config));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_index_end_to_end,
    bench_hash_aggregation,
    bench_process_file_ngrams
);
criterion_main!(benches);
