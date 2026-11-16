# rust-matching-engine

用 Rust 实现的限价单撮合引擎，包含订单簿、撮合、WAL 持久化和快照恢复。

## 功能

- 订单簿：价格优先、时间优先
- 订单类型：限价单、市价单
- 撤单
- 部分成交
- WAL 预写日志，崩溃后按序重放
- 快照，与 WAL 配合恢复订单簿状态
- 延迟统计：p50 / p99 / p999

## 性能

100,000 笔订单，release 构建：

```
orders:       100000
trades:       79728
resting:      20172
elapsed:      77.27ms
throughput:   1294163 orders/s
p50:          200 ns
p99:          900 ns
p999:         5600 ns
```

快照 + WAL 恢复一致性验证：

```
snapshot:     10087 orders
replayed:     50000
resting:      20172
last seq:     100000
```

## 运行

```bash
cargo run --release
cargo run --release -- demo 1000000
cargo run --release -- recover data
```

## 架构

```
input
  └── sequencer        定序，分配序号
        └── engine     撮合核心（单线程）
              ├── order book
              └── order index
        └── output     成交回报
        └── wal        预写日志
        └── snapshot   快照
```

撮合单线程执行，保证定序和状态一致；并发预留给输入解析、输出和持久化。

## 数据结构

- 买单：`BTreeMap<i64, VecDeque<u64>>`，价格降序取最优
- 卖单：`BTreeMap<i64, VecDeque<u64>>`，价格升序取最优
- 订单索引：`HashMap<u64, Order>`，撤单 O(1) 定位

## 设计取舍

**价格档位用 BTreeMap。** 需要按价格有序遍历取最优价，`HashMap` 做不到；数组在价格稀疏时会浪费空间。

**同价位用 VecDeque。** 保证 FIFO，时间优先。

**撮合单线程。** 多线程并行撮合会破坏定序，成交结果不确定。并发放在撮合之外。

**快照按订单簿顺序保存。** 如果按 `HashMap` 迭代顺序保存，恢复后同价位的 FIFO 顺序会被打乱，导致后续撮合结果偏离。

**WAL 和快照都要。** 只有 WAL，重启要重放全部历史；只有快照，两次快照之间的数据会丢。

## 限制

学习项目，不是生产系统。未实现：分布式部署、真实交易所协议、保证金和风控、多品种隔离。

## 开发文档

更详细的范围、里程碑和设计说明见 [DEVELOPMENT.md](DEVELOPMENT.md)。
