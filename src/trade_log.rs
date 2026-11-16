use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Write};
use std::io::BufRead;
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

pub fn count(path: &Path) -> io::Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let file = File::open(path)?;
    Ok(BufReader::new(file).lines().count())
}
