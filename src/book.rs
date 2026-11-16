use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::order::{Order, Side};

#[derive(Default)]
pub struct OrderBook {
    bids: BTreeMap<i64, VecDeque<u64>>,
    asks: BTreeMap<i64, VecDeque<u64>>,
    orders: HashMap<u64, Order>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn best_bid(&self) -> Option<i64> {
        self.bids.keys().next_back().copied()
    }

    pub fn best_ask(&self) -> Option<i64> {
        self.asks.keys().next().copied()
    }

    pub fn get(&self, id: u64) -> Option<&Order> {
        self.orders.get(&id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Order> {
        self.orders.get_mut(&id)
    }

    pub fn len(&self) -> usize {
        self.orders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    fn levels(&self, side: Side) -> &BTreeMap<i64, VecDeque<u64>> {
        match side {
            Side::Buy => &self.bids,
            Side::Sell => &self.asks,
        }
    }

    fn levels_mut(&mut self, side: Side) -> &mut BTreeMap<i64, VecDeque<u64>> {
        match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        }
    }

    pub fn front(&self, side: Side, price: i64) -> Option<u64> {
        self.levels(side).get(&price).and_then(|q| q.front().copied())
    }

    pub fn insert(&mut self, order: Order) {
        let side = order.side;
        let price = order.price;
        let id = order.id;
        self.orders.insert(id, order);
        self.levels_mut(side).entry(price).or_default().push_back(id);
    }

    pub fn remove(&mut self, id: u64) -> bool {
        let Some(order) = self.orders.remove(&id) else {
            return false;
        };
        let side = order.side;
        let price = order.price;
        let levels = self.levels_mut(side);
        if let Some(queue) = levels.get_mut(&price) {
            if let Some(pos) = queue.iter().position(|x| *x == id) {
                queue.remove(pos);
            }
            if queue.is_empty() {
                levels.remove(&price);
            }
        }
        true
    }

    pub fn snapshot_orders(&self) -> Vec<Order> {
        let mut out = Vec::with_capacity(self.orders.len());
        for queue in self.bids.values() {
            for id in queue {
                if let Some(o) = self.orders.get(id) {
                    out.push(o.clone());
                }
            }
        }
        for queue in self.asks.values() {
            for id in queue {
                if let Some(o) = self.orders.get(id) {
                    out.push(o.clone());
                }
            }
        }
        out
    }

    pub fn bid_levels(&self) -> usize {
        self.bids.len()
    }

    pub fn ask_levels(&self) -> usize {
        self.asks.len()
    }
}
