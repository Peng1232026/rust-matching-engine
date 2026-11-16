//! WAL（Write-Ahead Log，预写日志）。
//!
//! 思路：每收到一条命令，先追加写到日志文件，再执行撮合。
//! 这样程序崩溃后，可以按顺序重放日志把状态恢复出来。
//!
//! 为什么快：追加写是顺序 IO，磁盘只要一直往后写，不需要寻道。
//! 数据库的随机读写要慢一个数量级以上。

use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use crate::order::{Order, OrderKind, Side};

/// 一条可重放的命令。
#[derive(Clone, Debug)]
pub enum Command {
    Order { seq: u64, order: Order },
    Cancel { seq: u64, id: u64 },
}

/// 日志写入器。
///
/// 内部用 `BufWriter` 包住文件：每次 `writeln!` 先进内存缓冲，
/// 缓冲满了才真正写磁盘，避免每条命令一次系统调用。
pub struct Wal {
    file: BufWriter<File>,
}

impl Wal {
    pub fn open(path: &Path) -> io::Result<Self> {
        // append(true)：以追加模式打开，不会覆盖已有内容。
        // create(true)：文件不存在就创建。
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: BufWriter::new(file),
        })
    }

    /// 记录一笔下单。
    /// 格式：`O,seq,id,side,kind,price,qty,ts`
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

    /// 记录一笔撤单。格式：`C,seq,id`
    pub fn append_cancel(&mut self, seq: u64, id: u64) -> io::Result<()> {
        writeln!(self.file, "C,{},{}", seq, id)
    }

    /// 把缓冲区刷到磁盘，并调用 fsync 确保真正落地。
    ///
    /// 注意：`writeln!` 只是写进缓冲区，不 fsync 的话断电仍可能丢数据。
    pub fn sync(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.file.get_ref().sync_all()
    }
}

/// 读取命令文件，解析成 `Command` 列表。
///
/// 非法行直接跳过，不报错，方便手工编辑的文件。
pub fn read_commands(path: &Path) -> io::Result<Vec<Command>> {
    let mut out = Vec::new();
    if !path.exists() {
        return Ok(out);
    }
    let file = File::open(path)?;

    // `.lines()` 按行迭代，每行是一个 `io::Result<String>`。
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
