use criterion::{black_box, criterion_group, criterion_main, Criterion};
use matching_engine::engine::Engine;
use matching_engine::order::{Order, Side};

fn build_engine(n: u64) -> Engine {
    let mut e = Engine::new();
    for i in 0..n {
        let side = if i % 2 == 0 { Side::Buy } else { Side::Sell };
        let jitter = ((i.wrapping_mul(2654435761)) % 200) as i64 - 100;
        e.submit(Order::limit(i + 1, side, 10_000 + jitter, 100 + (i % 50), i));
    }
    e
}

fn bench_submit(c: &mut Criterion) {
    c.bench_function("submit_10k", |b| {
        b.iter(|| {
            let e = build_engine(10_000);
            black_box(e.trades().len())
        })
    });
}

fn bench_cancel(c: &mut Criterion) {
    let mut e = build_engine(10_000);
    c.bench_function("cancel_5k", |b| {
        b.iter(|| {
            for id in 1..=5_000u64 {
                e.cancel(id);
            }
            black_box(e.book().len())
        })
    });
}

fn bench_snapshot_orders(c: &mut Criterion) {
    let e = build_engine(10_000);
    c.bench_function("snapshot_orders_10k", |b| {
        b.iter(|| black_box(e.book().snapshot_orders()))
    });
}

criterion_group!(benches, bench_submit, bench_cancel, bench_snapshot_orders);
criterion_main!(benches);
