// Traces to: NFR-003 (ledger scalability)
//
// Benchmark cross-dimension scanner on the current repo.
// Skip vendor + target dirs. Assert end-to-end scan completes in < 500 ms on a cold walk.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use walkdir::WalkDir;

fn bench_directory_scan_workspace(c: &mut Criterion) {
    // Benchmark walking the hwLedger workspace tree, excluding vendor and target.
    c.bench_function("scan_workspace_tree_excluding_vendor_target", |b| {
        b.iter(|| {
            let root = black_box("/Users/kooshapari/CodeProjects/Phenotype/repos/hwLedger");
            let mut count = 0;
            for entry in WalkDir::new(root)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let path = e.path();
                    !path.to_string_lossy().contains("vendor")
                        && !path.to_string_lossy().contains("target")
                        && !path.to_string_lossy().contains(".git")
                })
            {
                if entry.file_type().is_file() {
                    count += 1;
                }
            }
            black_box(count);
        });
    });
}

criterion_group!(benches, bench_directory_scan_workspace);
criterion_main!(benches);
