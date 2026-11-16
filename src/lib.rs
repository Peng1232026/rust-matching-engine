//! 撮合引擎库。
//!
//! 模块划分：
//!
//! - `order`     订单类型
//! - `book`      订单簿
//! - `engine`    撮合核心
//! - `report`    成交回报
//! - `wal`       预写日志
//! - `snapshot`  快照
//! - `trade_log` 成交日志
//! - `metrics`   延迟统计

pub mod book;
pub mod engine;
pub mod metrics;
pub mod order;
pub mod report;
pub mod snapshot;
pub mod trade_log;
pub mod wal;
