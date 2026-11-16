//! 成交日志：把每笔成交单独写一份文件。
//!
//! 和 WAL 分开的原因：WAL 存的是"输入"，成交日志存的是"输出"。
//! 生产环境里成交日志用于对账、风控和下游推送。

use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use crate::order::Side;
use crate::report::Trade;

pub struct TradeLog {
    file: BufWriter<File>,
}

impl TradeLog {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: BufWriter::new(file),
        })
    }

    /// 追加一笔成交。格式：`taker,maker,price,qty,side`
    pub fn append(&mut self, trade: &Trade) -> io::Result<()> {
        let side = match trade.taker_side {
            Side::Buy => "B",
            Side::Sell => "S",
        };
        writeln!(
            self.file,
            "{},{},{},{},{}",
            trade.taker_id, trade.maker_id, trade.price, trade.qty, side
        )
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.file.get_ref().sync_all()
    }
}

/// 数一下成交日志有多少行，用来校验写入是否完整。
pub fn count(path: &Path) -> io::Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let file = File::open(path)?;
    Ok(BufReader::new(file).lines().count())
}
