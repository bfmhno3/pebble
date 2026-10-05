//! Criterion benchmarks comparing the tree-walking interpreter with the
//! bytecode VM on three workloads.
//!
//! Every workload runs both backends in the same benchmark group, so the
//! comparison is visible side by side in the report.

use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};

use pebble_bench::{COLLECTIONS, FIB, LOOP_SUM, run_interp, run_vm};

fn backends(c: &mut Criterion) {
    let programs = [
        ("fib", FIB),
        ("loop_sum", LOOP_SUM),
        ("collections", COLLECTIONS),
    ];

    for (name, src) in programs {
        let mut group = c.benchmark_group(name);
        group
            .sample_size(20)
            .measurement_time(Duration::from_secs(2));
        group.bench_function("interp", |b| b.iter(|| run_interp(src)));
        group.bench_function("vm", |b| b.iter(|| run_vm(src)));
        group.finish();
    }
}

criterion_group!(benches, backends);
criterion_main!(benches);
