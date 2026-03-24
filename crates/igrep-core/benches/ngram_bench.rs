use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use igrep_core::ngram::{build_all_ngrams, build_covering_ngrams, hash_ngram};
use igrep_core::posting::{decode_posting_list, encode_posting_list};
use igrep_core::types::IndexConfig;

fn bench_build_all(c: &mut Criterion) {
    let config = IndexConfig::default();
    let sizes = [100, 1000, 10000];
    let mut group = c.benchmark_group("build_all_ngrams");
    for size in sizes {
        let input: Vec<u8> = (0..size).map(|i| (i % 96 + 32) as u8).collect();
        group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, inp| {
            b.iter(|| build_all_ngrams(black_box(inp), &config));
        });
    }
    group.finish();
}

fn bench_build_covering(c: &mut Criterion) {
    let config = IndexConfig::default();
    let sizes = [100, 1000, 10000];
    let mut group = c.benchmark_group("build_covering_ngrams");
    for size in sizes {
        let input: Vec<u8> = (0..size).map(|i| (i % 96 + 32) as u8).collect();
        group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, inp| {
            b.iter(|| build_covering_ngrams(black_box(inp), &config));
        });
    }
    group.finish();
}

fn bench_posting_roundtrip(c: &mut Criterion) {
    let doc_ids: Vec<u32> = (0..1000).step_by(3).collect();
    c.bench_function("posting_encode_decode_1000", |b| {
        b.iter(|| {
            let mut buf = Vec::new();
            encode_posting_list(black_box(&doc_ids), &mut buf);
            let mut pos = 0;
            let _ = decode_posting_list(black_box(&buf), &mut pos);
        });
    });
}

fn bench_hash_ngram(c: &mut Criterion) {
    let input = b"hello world test";
    c.bench_function("hash_ngram", |b| {
        b.iter(|| hash_ngram(black_box(input)));
    });
}

criterion_group!(
    benches,
    bench_build_all,
    bench_build_covering,
    bench_posting_roundtrip,
    bench_hash_ngram
);
criterion_main!(benches);
