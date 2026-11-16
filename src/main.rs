//! 命令行入口。
//!
//! 四个子命令：
//!
//! - `demo [n]`     生成 n 条合成命令跑一遍，写 WAL、快照、成交日志
//! - `run <file>`   回放命令文件
//! - `recover <dir>` 从快照 + WAL 恢复
//! - `pipeline [n]` 三线程流水线，测并发吞吐

use std::path::{Path, PathBuf};
use std::time::Instant;

use matching_engine::engine::Engine;
use matching_engine::metrics::LatencyStats;
use matching_engine::order::{Order, Side};
use matching_engine::report::Trade;
use matching_engine::snapshot;
use matching_engine::trade_log::TradeLog;
use matching_engine::wal::{self, Command, Wal};

fn main() {
    // std::env::args() 第一个元素是程序路径，所以参数从下标 1 开始。
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()).unwrap_or("demo") {
        "run" => {
            let path = PathBuf::from(args.get(2).expect("usage: run <commands.csv>"));
            run_file(&path);
        }
        "recover" => {
            let dir = PathBuf::from(args.get(2).map(|s| s.as_str()).unwrap_or("data"));
            recover(&dir);
        }
        "pipeline" => {
            let total = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100_000);
            pipeline(total);
        }
        _ => {
            let total = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(100_000);
            demo(total);
        }
    }
}

/// 生成合成命令，完整跑一遍撮合，并把数据落盘。
fn demo(total: u64) {
    let dir = PathBuf::from("data");
    std::fs::create_dir_all(&dir).unwrap();

    let wal_path = dir.join("wal.log");
    let snap_path = dir.join("snapshot.txt");
    let trade_path = dir.join("trades.log");

    // 开跑前清掉上次的产物，保证每次结果独立。
    let _ = std::fs::remove_file(&wal_path);
    let _ = std::fs::remove_file(&snap_path);
    let _ = std::fs::remove_file(&trade_path);

    let mut engine = Engine::new();
    let mut wal = Wal::open(&wal_path).unwrap();
    let mut trade_log = TradeLog::open(&trade_path).unwrap();
    let mut stats = LatencyStats::with_capacity(total as usize);

    // pending 记录当前还挂在簿上的订单号，撤单时从这里挑。
    let mut pending: Vec<u64> = Vec::new();
    let mut cancels = 0u64;

    let start = Instant::now();

    for i in 0..total {
        let seq = engine.next_seq();

        // 每 100 条命令里放 1 条撤单，模拟真实流量。
        if i % 100 == 99 {
            if let Some(id) = pending.pop() {
                wal.append_cancel(seq, id).unwrap();
                engine.cancel(id);
                cancels += 1;
            }
        } else {
            let order = synthetic(i);
            let id = order.id;
            wal.append_order(seq, &order).unwrap();

            // 撮合前记下成交条数，撮合后这一段就是本次新增的成交。
            let before = engine.trades().len();

            let t0 = Instant::now();
            engine.submit(order);
            stats.record(t0.elapsed().as_nanos() as u64);

            for t in &engine.trades()[before..] {
                trade_log.append(t).unwrap();
            }

            // 没成交完的部分会挂进订单簿，记下来等后面撤单。
            if engine.book().get(id).is_some() {
                pending.push(id);
            }
        }

        // 跑到一半存一个快照，后面恢复时只需要重放后半段 WAL。
        // 注意必须放在命令执行之后：如果放在之前，这条命令的序号会被算进快照，
        // 恢复时就被跳过，状态会差一条。
        if i + 1 == total / 2 {
            snapshot::save(&snap_path, seq, &engine.book().snapshot_orders()).unwrap();
        }
    }

    wal.sync().unwrap();
    trade_log.sync().unwrap();
    let elapsed = start.elapsed();

    println!("orders:       {}", total - cancels);
    println!("cancels:      {}", cancels);
    println!("trades:       {}", engine.trades().len());
    println!("resting:      {}", engine.book().len());
    println!("bid levels:   {}", engine.book().bid_levels());
    println!("ask levels:   {}", engine.book().ask_levels());
    println!("elapsed:      {:?}", elapsed);
    println!(
        "throughput:   {:.0} ops/s",
        total as f64 / elapsed.as_secs_f64()
    );
    println!("p50:          {} ns", stats.percentile(0.50));
    println!("p99:          {} ns", stats.percentile(0.99));
    println!("p999:         {} ns", stats.percentile(0.999));
}

/// 生成第 i 条合成订单。
///
/// 价格围绕 10000 上下抖动，买卖混在一起，会产生大量交叉成交。
/// 抖动用乘法和取模做伪随机，避免引入 rand 依赖。
fn synthetic(i: u64) -> Order {
    let id = i + 1;
    let side = if i % 2 == 0 { Side::Buy } else { Side::Sell };

    // 每 50 条里放一笔市价单。
    if i % 50 == 25 {
        return Order::market(id, side, 50, i);
    }

    let mid = 10_000i64;
    let jitter = ((i.wrapping_mul(2654435761)) % 200) as i64 - 100;
    Order::limit(id, side, mid + jitter, 100 + (i % 50), i)
}

/// 三线程流水线：
///
/// - 主线程：生成命令，发到 channel
/// - 撮合线程：从 channel 收命令，执行撮合，把成交发到另一个 channel
/// - 写出线程：收成交，写文件
///
/// 撮合仍然是单线程，并发只在它前后。这是交易系统的常见结构：
/// 定序不能并行，但 IO 可以。
fn pipeline(total: u64) {
    use std::sync::mpsc;
    use std::thread;

    std::fs::create_dir_all("data").unwrap();

    // mpsc = multi-producer, single-consumer，标准库的多生产者单消费者通道。
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
    let (trade_tx, trade_rx) = mpsc::channel::<Trade>();

    // 写出线程。`move` 把 trade_rx 的所有权移进闭包。
    let writer = thread::spawn(move || {
        let path = PathBuf::from("data/pipeline_trades.log");
        let _ = std::fs::remove_file(&path);
        let mut log = TradeLog::open(&path).unwrap();
        let mut n = 0usize;
        // 通道关闭后 for 循环自动结束。
        for t in trade_rx {
            log.append(&t).unwrap();
            n += 1;
        }
        log.sync().unwrap();
        n
    });

    // 撮合线程。它独占 Engine，不需要锁。
    let matcher = thread::spawn(move || {
        let mut engine = Engine::new();
        for cmd in cmd_rx {
            match cmd {
                Command::Order { seq, order } => {
                    engine.set_seq(seq);
                    let before = engine.trades().len();
                    engine.submit(order);
                    for t in &engine.trades()[before..] {
                        trade_tx.send(t.clone()).unwrap();
                    }
                }
                Command::Cancel { seq, id } => {
                    engine.set_seq(seq);
                    engine.cancel(id);
                }
            }
        }
        // 闭包返回撮合线程的统计值，join 时取回。
        (engine.trades().len(), engine.book().len())
    });

    let start = Instant::now();
    for i in 0..total {
        let seq = i + 1;
        let cmd = if i % 100 == 99 {
            Command::Cancel { seq, id: i }
        } else {
            Command::Order {
                seq,
                order: synthetic(i),
            }
        };
        cmd_tx.send(cmd).unwrap();
    }
    // 主动 drop 发送端，撮合线程的 for 循环才会结束。
    drop(cmd_tx);

    let (trades, resting) = matcher.join().unwrap();
    let written = writer.join().unwrap();
    let elapsed = start.elapsed();

    println!("commands:     {}", total);
    println!("trades:       {}", trades);
    println!("written:      {}", written);
    println!("resting:      {}", resting);
    println!("elapsed:      {:?}", elapsed);
    println!(
        "throughput:   {:.0} ops/s",
        total as f64 / elapsed.as_secs_f64()
    );
}

/// 回放一个命令文件。
fn run_file(path: &Path) {
    let commands = wal::read_commands(path).unwrap();
    let mut engine = Engine::new();
    let mut stats = LatencyStats::with_capacity(commands.len());
    let mut cancels = 0usize;

    let start = Instant::now();
    for cmd in &commands {
        let t0 = Instant::now();
        match cmd {
            Command::Order { seq, order } => {
                engine.set_seq(*seq);
                engine.submit(order.clone());
            }
            Command::Cancel { seq, id } => {
                engine.set_seq(*seq);
                engine.cancel(*id);
                cancels += 1;
            }
        }
        stats.record(t0.elapsed().as_nanos() as u64);
    }
    let elapsed = start.elapsed();

    println!("commands:     {}", commands.len());
    println!("cancels:      {}", cancels);
    println!("trades:       {}", engine.trades().len());
    println!("resting:      {}", engine.book().len());
    println!("elapsed:      {:?}", elapsed);
    println!(
        "throughput:   {:.0} ops/s",
        commands.len() as f64 / elapsed.as_secs_f64()
    );
    println!("p99:          {} ns", stats.percentile(0.99));
}

/// 从快照 + WAL 恢复。
///
/// 步骤：
/// 1. 加载快照，把挂单直接放回订单簿（不再撮合）
/// 2. 读 WAL，跳过序号小于等于快照序号的命令
/// 3. 重放剩余命令
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
    let mut cancels = 0usize;

    for cmd in wal::read_commands(&wal_path).unwrap() {
        match cmd {
            Command::Order { seq, order } => {
                // 快照已经包含到 seq 为止的状态，之前的命令不能再执行。
                if seq <= engine.seq() {
                    continue;
                }
                engine.submit(order);
                engine.set_seq(seq);
                replayed += 1;
            }
            Command::Cancel { seq, id } => {
                if seq <= engine.seq() {
                    continue;
                }
                engine.cancel(id);
                engine.set_seq(seq);
                cancels += 1;
            }
        }
    }

    println!("snapshot:     {} orders", from_snapshot);
    println!("replayed:     {} orders", replayed);
    println!("cancels:      {}", cancels);
    println!("resting:      {}", engine.book().len());
    println!("trades:       {}", engine.trades().len());
    println!("last seq:     {}", engine.seq());
}
