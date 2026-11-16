use std::path::{Path, PathBuf};
use std::time::Instant;

use matching_engine::engine::Engine;
use matching_engine::metrics::LatencyStats;
use matching_engine::order::{Order, Side};
use matching_engine::wal::Wal;
use matching_engine::{snapshot, wal};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()).unwrap_or("demo") {
        "run" => {
            let path = PathBuf::from(args.get(2).expect("usage: run <orders.csv>"));
            run_file(&path);
        }
        "recover" => {
            let dir = PathBuf::from(args.get(2).map(|s| s.as_str()).unwrap_or("data"));
            recover(&dir);
        }
        _ => {
            let total = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100_000);
            demo(total);
        }
    }
}

fn demo(total: u64) {
    let dir = PathBuf::from("data");
    std::fs::create_dir_all(&dir).unwrap();
    let wal_path = dir.join("wal.log");
    let snap_path = dir.join("snapshot.txt");
    let _ = std::fs::remove_file(&wal_path);
    let _ = std::fs::remove_file(&snap_path);

    let mut engine = Engine::new();
    let mut wal = Wal::open(&wal_path).unwrap();
    let mut stats = LatencyStats::with_capacity(total as usize);

    let start = Instant::now();
    for i in 0..total {
        let order = synthetic(i);
        let seq = engine.next_seq();
        wal.append(seq, &order).unwrap();

        let t0 = Instant::now();
        engine.submit(order);
        stats.record(t0.elapsed().as_nanos() as u64);

        if i + 1 == total / 2 {
            snapshot::save(&snap_path, seq, &engine.book().snapshot_orders()).unwrap();
        }
    }
    wal.sync().unwrap();
    let elapsed = start.elapsed();

    println!("orders:       {}", total);
    println!("trades:       {}", engine.trades().len());
    println!("resting:      {}", engine.book().len());
    println!("bid levels:   {}", engine.book().bid_levels());
    println!("ask levels:   {}", engine.book().ask_levels());
    println!("elapsed:      {:?}", elapsed);
    println!(
        "throughput:   {:.0} orders/s",
        total as f64 / elapsed.as_secs_f64()
    );
    println!("p50:          {} ns", stats.percentile(0.50));
    println!("p99:          {} ns", stats.percentile(0.99));
    println!("p999:         {} ns", stats.percentile(0.999));
}

fn synthetic(i: u64) -> Order {
    let id = i + 1;
    let side = if i % 2 == 0 { Side::Buy } else { Side::Sell };
    let mid = 10_000i64;
    let jitter = ((i.wrapping_mul(2654435761)) % 200) as i64 - 100;
    let price = mid + jitter;
    Order::limit(id, side, price, 100 + (i % 50), i)
}

fn run_file(path: &Path) {
    let orders = wal::read_orders(path).unwrap();
    let mut engine = Engine::new();
    let mut stats = LatencyStats::with_capacity(orders.len());

    let start = Instant::now();
    for (_, order) in &orders {
        let t0 = Instant::now();
        engine.submit(order.clone());
        stats.record(t0.elapsed().as_nanos() as u64);
    }
    let elapsed = start.elapsed();

    println!("orders:       {}", orders.len());
    println!("trades:       {}", engine.trades().len());
    println!("resting:      {}", engine.book().len());
    println!("elapsed:      {:?}", elapsed);
    println!(
        "throughput:   {:.0} orders/s",
        orders.len() as f64 / elapsed.as_secs_f64()
    );
    println!("p99:          {} ns", stats.percentile(0.99));
}

fn recover(dir: &Path) {
    let snap_path = dir.join("snapshot.txt");
    let wal_path = dir.join("wal.log");

    let mut engine = Engine::new();
    let mut from_snapshot = 0usize;

    if let Some((seq, orders)) = snapshot::load(&snap_path).unwrap() {
        from_snapshot = orders.len();
        for order in orders {
            engine.restore(order);
        }
        engine.set_seq(seq);
    }

    let mut replayed = 0usize;
    for (seq, order) in wal::read_orders(&wal_path).unwrap() {
        if seq <= engine.seq() {
            continue;
        }
        engine.submit(order);
        engine.set_seq(seq);
        replayed += 1;
    }

    println!("snapshot:     {} orders", from_snapshot);
    println!("replayed:     {}", replayed);
    println!("resting:      {}", engine.book().len());
    println!("trades:       {}", engine.trades().len());
    println!("last seq:     {}", engine.seq());
}
