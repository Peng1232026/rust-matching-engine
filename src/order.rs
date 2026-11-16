#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderKind {
    Limit,
    Market,
}

#[derive(Clone, Debug)]
pub struct Order {
    pub id: u64,
    pub price: i64,
    pub qty: u64,
    pub side: Side,
    pub kind: OrderKind,
    pub ts: u64,
}

impl Order {
    pub fn limit(id: u64, side: Side, price: i64, qty: u64, ts: u64) -> Self {
        Self {
            id,
            price,
            qty,
            side,
            kind: OrderKind::Limit,
            ts,
        }
    }

    pub fn market(id: u64, side: Side, qty: u64, ts: u64) -> Self {
        Self {
            id,
            price: 0,
            qty,
            side,
            kind: OrderKind::Market,
            ts,
        }
    }
}
