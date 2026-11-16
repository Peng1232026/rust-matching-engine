use crate::order::Side;

#[derive(Clone, Debug)]
pub struct Trade {
    pub taker_id: u64,
    pub maker_id: u64,
    pub price: i64,
    pub qty: u64,
    pub taker_side: Side,
}
