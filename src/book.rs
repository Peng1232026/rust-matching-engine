//! 订单簿：维护所有未成交的挂单。
//!
//! 三个数据结构的组合：
//!
//! - `BTreeMap<i64, VecDeque<u64>>`：价格 -> 该价格下的订单号队列，按价格有序
//! - `HashMap<u64, Order>`：订单号 -> 订单，用于 O(1) 查找和撤单
//!
//! `BTreeMap` 是 Rust 标准库里的有序映射，内部是 B 树。它的 `keys()` 遍历是排好序的，
//! 所以取最优价不用每次扫描。Java 里对应 `TreeMap`。
//!
//! `VecDeque` 是双端队列，两头都能高效地进出。用作队列时遵守先进先出，
//! 正好实现"同价位先挂单先成交"的时间优先。

use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::order::{Order, Side};

/// 订单簿：买单和卖单分开存。
///
/// `#[derive(Default)]` 自动生成一个全空的构造函数，
/// 因为 BTreeMap、HashMap 本身都实现了 Default。
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

    /// 最优买价：买单里价格最高的那个。
    /// `next_back()` 取 BTreeMap 最后一个键，因为 BTreeMap 是升序的。
    pub fn best_bid(&self) -> Option<i64> {
        self.bids.keys().next_back().copied()
    }

    /// 最优卖价：卖单里价格最低的那个。
    /// `next()` 取第一个键。
    pub fn best_ask(&self) -> Option<i64> {
        self.asks.keys().next().copied()
    }

    pub fn get(&self, id: u64) -> Option<&Order> {
        self.orders.get(&id)
    }

    /// 返回可变引用，调用方可以直接修改订单数量（比如部分成交后扣减）。
    ///
    /// `&mut self` 表示这个方法需要独占借用，调用期间不能再有别的引用。
    pub fn get_mut(&mut self, id: u64) -> Option<&mut Order> {
        self.orders.get_mut(&id)
    }

    pub fn len(&self) -> usize {
        self.orders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }

    /// 只读访问某一侧的价位表。返回引用，不复制数据。
    fn levels(&self, side: Side) -> &BTreeMap<i64, VecDeque<u64>> {
        match side {
            Side::Buy => &self.bids,
            Side::Sell => &self.asks,
        }
    }

    /// 可变访问某一侧的价位表。
    /// 这里要用 `&mut self`，返回 `&mut BTreeMap`，借用检查器保证同一时刻只有一个可变借用。
    fn levels_mut(&mut self, side: Side) -> &mut BTreeMap<i64, VecDeque<u64>> {
        match side {
            Side::Buy => &mut self.bids,
            Side::Sell => &mut self.asks,
        }
    }

    /// 取某个价位上排队最靠前的订单号。
    /// `front()` 是 VecDeque 的队头，即最早挂进来的那笔。
    pub fn front(&self, side: Side, price: i64) -> Option<u64> {
        self.levels(side).get(&price).and_then(|q| q.front().copied())
    }

    /// 把订单放进订单簿。
    ///
    /// 参数 `order: Order` 是值传递，会移动所有权进来，之后调用方不能再使用这个 order。
    /// 如果确实要传值，这是 Rust 的默认行为；不想移动就传 `&Order`。
    pub fn insert(&mut self, order: Order) {
        let side = order.side;
        let price = order.price;
        let id = order.id;

        // 先存进索引表。`insert` 返回旧值，这里不需要。
        self.orders.insert(id, order);

        // `entry(price).or_default()`：如果这个价位不存在就插入一个空队列。
        // 然后 `push_back` 排队尾，保证 FIFO。
        self.levels_mut(side).entry(price).or_default().push_back(id);
    }

    /// 撤单：从索引表和价位队列里同时删掉。
    /// 返回 true 表示删掉了，false 表示订单不存在。
    pub fn remove(&mut self, id: u64) -> bool {
        // `let ... else` 是 Rust 1.65 的写法：不匹配就直接 return false。
        let Some(order) = self.orders.remove(&id) else {
            return false;
        };

        let side = order.side;
        let price = order.price;
        let levels = self.levels_mut(side);

        if let Some(queue) = levels.get_mut(&price) {
            // position 找到订单在队列里的下标，remove 按位置删除。
            // VecDeque 中间删除是 O(n)，但同一价位的订单通常不多。
            if let Some(pos) = queue.iter().position(|x| *x == id) {
                queue.remove(pos);
            }
            // 这个价位已经空了，把整个键删掉，避免 BTreeMap 里留下空档。
            if queue.is_empty() {
                levels.remove(&price);
            }
        }
        true
    }

    /// 导出所有挂单，用于写快照。
    ///
    /// 注意这里按价位顺序遍历，而不是直接遍历 `orders`：
    /// HashMap 的迭代顺序是随机的，按它导出会打乱同价位的 FIFO 顺序，
    /// 恢复之后撮合结果就会偏离。
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
