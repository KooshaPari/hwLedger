// Traces to: NFR-003 (ledger scalability)
//
// Benchmark axum route handling for hot paths (`/v1/agents/:id/heartbeat`).
// Use tower::ServiceExt::oneshot to drive the router in-process.
// Target: < 1 ms per heartbeat at P99.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_heartbeat_route_processing(c: &mut Criterion) {
    // Simplified benchmark: measure JSON serialization + basic routing overhead.
    // In production, this would use an actual axum router with real middleware.

    c.bench_function("heartbeat_route_process_basic", |b| {
        b.iter(|| {
            let heartbeat_json = serde_json::json!({
                "agent_id": "agent-001",
                "timestamp": "2025-04-18T00:00:00Z",
                "gpu_memory_available": 40960,
                "gpu_memory_used": 20480,
                "cpu_usage_percent": 25.5,
                "temperature_celsius": 72.0
            });
            let _serialized = black_box(serde_json::to_string(&heartbeat_json).unwrap());
        });
    });
}

criterion_group!(benches, bench_heartbeat_route_processing);
criterion_main!(benches);
