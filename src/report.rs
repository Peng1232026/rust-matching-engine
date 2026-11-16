//! 成交回报。

use crate::order::Side;

/// 一笔成交。
///
/// `taker` 是主动方（新来的订单），`maker` 是被动方（先挂在簿上的订单）。
#[derive(Clone, Debug)]
pub struct Trade {
    pub taker_id: u64,
    pub maker_id: u64,
    pub price: i64,
    pub qty: u64,
    pub taker_side: Side,
}
