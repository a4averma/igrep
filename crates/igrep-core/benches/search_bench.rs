use criterion::{black_box, criterion_group, criterion_main, Criterion};
use igrep_core::query::regex_to_query;

fn bench_regex_decomposition(c: &mut Criterion) {
    let patterns = [
        "fn main",
        "TODO|FIXME",
        r"struct\s+\w+",
        "MAX_FILE_SIZE",
        r"pub\s+(fn|struct)\s+\w+",
    ];
    for pat in patterns {
        c.bench_function(&format!("regex_decompose_{}", pat), |b| {
            b.iter(|| regex_to_query(black_box(pat)));
        });
    }
}

criterion_group!(benches, bench_regex_decomposition);
criterion_main!(benches);
