//! 订单的基础类型定义。
//!
//! 这个文件只有结构体和枚举，对应教程第 5、6 章的内容。

/// 买卖方向。
///
/// `Copy` 表示这个类型赋值时按位复制，不发生所有权转移。
/// Java 里 enum 是引用类型，Rust 的枚举默认是值类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

/// 订单类型。
///
/// 限价单（Limit）带价格，只在价格满足时才成交；
/// 市价单（Market）不带价格，立刻吃掉对手方最优价。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderKind {
    Limit,
    Market,
}

/// 一笔订单。
///
/// 价格用 `i64` 而不是 `f64`：浮点数有精度误差，撮合的价格必须精确。
/// 单位由调用方约定，比如 1 表示 0.01 元。
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
    /// 构造限价单。
    ///
    /// 返回类型写 `Self`，等价于 `Order`，是 Rust 的习惯写法。
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

    /// 构造市价单。市价单没有价格，`price` 置 0 占位。
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
