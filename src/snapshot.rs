use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use crate::order::{Order, OrderKind, Side};

pub fn save(path: &Path, seq: u64, orders: &[Order]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let f = File::create(&tmp)?;
        let mut w = BufWriter::new(f);
        writeln!(w, "{}", seq)?;
        for o in orders {
            let side = match o.side {
                Side::Buy => "B",
                Side::Sell => "S",
            };
            let kind = match o.kind {
                OrderKind::Limit => "L",
                OrderKind::Market => "M",
            };
            writeln!(w, "{},{},{},{},{},{}", o.id, side, kind, o.price, o.qty, o.ts)?;
        }
        w.flush()?;
        w.get_ref().sync_all()?;
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(tmp, path)
}

pub fn load(path: &Path) -> io::Result<Option<(u64, Vec<Order>)>> {
    if !path.exists() {
        return Ok(None);
    }
    let file = File::open(path)?;
    let mut lines = BufReader::new(file).lines();
    let seq: u64 = match lines.next() {
        Some(l) => l?.trim().parse().unwrap_or(0),
        None => 0,
    };
    let mut orders = Vec::new();
    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != 6 {
            continue;
        }
        let id: u64 = parts[0].parse().unwrap_or(0);
        let side = if parts[1] == "B" {
            Side::Buy
        } else {
            Side::Sell
        };
        let kind = if parts[2] == "L" {
            OrderKind::Limit
        } else {
            OrderKind::Market
        };
        let price: i64 = parts[3].parse().unwrap_or(0);
        let qty: u64 = parts[4].parse().unwrap_or(0);
        let ts: u64 = parts[5].parse().unwrap_or(0);
        orders.push(Order {
            id,
            price,
            qty,
            side,
            kind,
            ts,
        });
    }
    Ok(Some((seq, orders)))
}
