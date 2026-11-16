//! 撮合核心。
//!
//! 整个引擎是单线程的：所有订单按顺序进来，撮合完再处理下一笔。
//! 这样成交结果完全确定，不会因为线程调度而出现不同结果。

use crate::book::OrderBook;
use crate::order::{Order, OrderKind, Side};
use crate::report::Trade;

/// 撮合引擎：订单簿 + 序号 + 成交记录。
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

    /// 返回成交记录的切片。
    /// `&[Trade]` 是只读视图，调用方拿不到所有权，不能修改。
    pub fn trades(&self) -> &[Trade] {
        &self.trades
    }

    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// 恢复时把序号直接设成快照里的值，不用从 0 重新数。
    pub fn set_seq(&mut self, seq: u64) {
        self.seq = seq;
    }

    pub fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    /// 恢复快照时用：把订单直接放进订单簿，不做撮合。
    ///
    /// 因为快照里存的已经是撮合后的剩余挂单，再撮合一遍就错了。
    pub fn restore(&mut self, order: Order) {
        self.book.insert(order);
    }

    /// 提交一笔订单，返回产生的成交笔数。
    ///
    /// 参数是 `mut order: Order`：拿到所有权，并且在函数内可以修改（扣减数量）。
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

    /// 买单撮合：从最优卖价开始，一直吃到价格不满足为止。
    fn match_buy(&mut self, order: &mut Order) {
        loop {
            // 数量撮完了，结束。
            if order.qty == 0 {
                return;
            }

            // 对手方没有卖单，无法成交。
            // `let ... else` 拿不到值就跳出循环。
            let Some(ask) = self.book.best_ask() else {
                break;
            };

            // 限价单：卖价高于买价，说明买不起，停止。
            // 市价单没有价格限制，会一直吃到底。
            if order.kind == OrderKind::Limit && ask > order.price {
                break;
            }

            // 取这个价位上排队最靠前的卖单。
            let Some(resting_id) = self.book.front(Side::Sell, ask) else {
                break;
            };

            // 先读出对方订单的数量和价格，再执行。
            // 这里的作用域限制很关键：`resting` 是 `&Order`（不可变借用），
            // 借用在花括号结束时释放，后面才能调用 `&mut self` 的方法。
            let Some(resting) = self.book.get(resting_id) else {
                break;
            };
            let fill = order.qty.min(resting.qty);
            let price = resting.price;

            self.execute(order, resting_id, fill, price);
        }

        // 还有剩余数量，挂进订单簿等对手方。
        // 市价单不挂单，吃不完就作废。
        if order.qty > 0 && order.kind == OrderKind::Limit {
            self.book.insert(order.clone());
        }
    }

    /// 卖单撮合，逻辑和买单对称。
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

    /// 执行一笔成交：扣减双方数量，对方吃完就移出订单簿，记录成交。
    ///
    /// 成交价用被动方（先挂单方）的价格，这是交易所的通行做法。
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
