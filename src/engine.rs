use crate::book::OrderBook;
use crate::order::{Order, OrderKind, Side};
use crate::report::Trade;

pub struct Engine {
    book: OrderBook,
    seq: u64,
    trades: Vec<Trade>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            book: OrderBook::new(),
            seq: 0,
            trades: Vec::new(),
        }
    }

    pub fn book(&self) -> &OrderBook {
        &self.book
    }

    pub fn trades(&self) -> &[Trade] {
        &self.trades
    }

    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn set_seq(&mut self, seq: u64) {
        self.seq = seq;
    }

    pub fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    pub fn restore(&mut self, order: Order) {
        self.book.insert(order);
    }

    pub fn submit(&mut self, mut order: Order) -> usize {
        let before = self.trades.len();
        match order.side {
            Side::Buy => self.match_buy(&mut order),
            Side::Sell => self.match_sell(&mut order),
        }
        self.trades.len() - before
    }

    pub fn cancel(&mut self, id: u64) -> bool {
        self.book.remove(id)
    }

    fn match_buy(&mut self, order: &mut Order) {
        loop {
            if order.qty == 0 {
                return;
            }
            let Some(ask) = self.book.best_ask() else {
                break;
            };
            if order.kind == OrderKind::Limit && ask > order.price {
                break;
            }
            let Some(resting_id) = self.book.front(Side::Sell, ask) else {
                break;
            };
            let Some(resting) = self.book.get(resting_id) else {
                break;
            };
            let fill = order.qty.min(resting.qty);
            let price = resting.price;
            self.execute(order, resting_id, fill, price);
        }
        if order.qty > 0 && order.kind == OrderKind::Limit {
            self.book.insert(order.clone());
        }
    }

    fn match_sell(&mut self, order: &mut Order) {
        loop {
            if order.qty == 0 {
                return;
            }
            let Some(bid) = self.book.best_bid() else {
                break;
            };
            if order.kind == OrderKind::Limit && bid < order.price {
                break;
            }
            let Some(resting_id) = self.book.front(Side::Buy, bid) else {
                break;
            };
            let Some(resting) = self.book.get(resting_id) else {
                break;
            };
            let fill = order.qty.min(resting.qty);
            let price = resting.price;
            self.execute(order, resting_id, fill, price);
        }
        if order.qty > 0 && order.kind == OrderKind::Limit {
            self.book.insert(order.clone());
        }
    }

    fn execute(&mut self, taker: &mut Order, resting_id: u64, qty: u64, price: i64) {
        taker.qty -= qty;
        let resting_done = {
            let Some(resting) = self.book.get_mut(resting_id) else {
                return;
            };
            resting.qty -= qty;
            resting.qty == 0
        };
        if resting_done {
            self.book.remove(resting_id);
        }
        self.trades.push(Trade {
            taker_id: taker.id,
            maker_id: resting_id,
            price,
            qty,
            taker_side: taker.side,
        });
    }
}
