use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn gguf_parse_bench(c: &mut Criterion) {
    c.bench_function("gguf_parse_stub", |b| {
        b.iter(|| black_box(1 + 1))
    });
}

criterion_group!(benches, gguf_parse_bench);
criterion_main!(benches);
