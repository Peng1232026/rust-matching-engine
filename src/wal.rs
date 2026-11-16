use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use crate::order::{Order, OrderKind, Side};

#[derive(Clone, Debug)]
pub enum Command {
    Order { seq: u64, order: Order },
    Cancel { seq: u64, id: u64 },
}

pub struct Wal {
    file: BufWriter<File>,
}

impl Wal {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: BufWriter::new(file),
        })
    }

    pub fn append_order(&mut self, seq: u64, order: &Order) -> io::Result<()> {
        let side = match order.side {
            Side::Buy => "B",
            Side::Sell => "S",
        };
        let kind = match order.kind {
            OrderKind::Limit => "L",
            OrderKind::Market => "M",
        };
        writeln!(
            self.file,
            "O,{},{},{},{},{},{},{}",
            seq, order.id, side, kind, order.price, order.qty, order.ts
        )
    }

    pub fn append_cancel(&mut self, seq: u64, id: u64) -> io::Result<()> {
        writeln!(self.file, "C,{},{}", seq, id)
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.file.get_ref().sync_all()
    }
}

pub fn read_commands(path: &Path) -> io::Result<Vec<Command>> {
    let mut out = Vec::new();
    if !path.exists() {
        return Ok(out);
    }
    let file = File::open(path)?;
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        match parts[0] {
            "O" if parts.len() == 8 => {
                let seq: u64 = parts[1].parse().unwrap_or(0);
                let id: u64 = parts[2].parse().unwrap_or(0);
                let side = if parts[3] == "B" { Side::Buy } else { Side::Sell };
                let kind = if parts[4] == "L" {
                    OrderKind::Limit
                } else {
                    OrderKind::Market
                };
                let price: i64 = parts[5].parse().unwrap_or(0);
                let qty: u64 = parts[6].parse().unwrap_or(0);
                let ts: u64 = parts[7].parse().unwrap_or(0);
                out.push(Command::Order {
                    seq,
                    order: Order {
                        id,
                        price,
                        qty,
                        side,
                        kind,
                        ts,
                    },
                });
            }
            "C" if parts.len() == 3 => {
                let seq: u64 = parts[1].parse().unwrap_or(0);
                let id: u64 = parts[2].parse().unwrap_or(0);
                out.push(Command::Cancel { seq, id });
            }
            _ => continue,
        }
    }
    Ok(out)
}
