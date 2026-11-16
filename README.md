# rust-matching-engine

用 Rust 实现的限价单撮合引擎，包含订单簿、撮合、WAL 持久化、快照恢复和并发流水线。

## 功能

- 订单簿：价格优先、时间优先
- 订单类型：限价单、市价单
- 撤单
- 部分成交
- WAL 预写日志，崩溃后按序重放
- 快照，与 WAL 配合恢复订单簿状态
- 成交回报单独落盘
- 并发流水线：输入、撮合、输出三线程
- 延迟统计：p50 / p99 / p999
- 9 个集成测试 + criterion 基准

## 性能

100,000 条命令（含撤单和市价单），release 构建，单线程：

```
orders:       99000
cancels:      1000
trades:       79787
resting:      18674
elapsed:      99.79ms
throughput:   1002088 ops/s
p50:          200 ns
p99:          800 ns
p999:         2900 ns
```

三线程流水线：

```
commands:     100000
trades:       79787
written:      79787
elapsed:      30.61ms
throughput:   3266629 ops/s
```

恢复一致性（快照 + WAL 重放后与正常运行对比）：

```
snapshot:     9338 orders
replayed:     49500 orders
cancels:      500
resting:      18674
last seq:     100000
```

criterion 基准：

```
submit_10k              time:   [1.8015 ms 1.8211 ms 1.8423 ms]
cancel_5k               time:   [64.598 µs 65.132 µs 65.695 µs]
snapshot_orders_10k     time:   [31.168 µs 31.539 µs 31.967 µs]
```

## 运行

```bash
cargo run --release                          # 10 万条命令，写 WAL/快照/成交日志
cargo run --release -- demo 1000000          # 指定数量
cargo run --release -- run examples/sample_commands.csv   # 回放命令文件
cargo run --release -- recover data          # 从快照 + WAL 恢复
cargo run --release -- pipeline 100000       # 三线程流水线
cargo test                                   # 集成测试
cargo bench                                  # criterion 基准
```

## 命令文件格式

```
O,seq,id,side,kind,price,qty,ts     # 下单，side=B/S，kind=L/M
C,seq,id                            # 撤单
```

WAL 使用同一格式，因此 `data/wal.log` 可以直接用 `run` 回放。

## 架构

```
input
  └── sequencer        定序，分配序号
        └── engine     撮合核心（单线程）
              ├── order book
              └── order index
        └── output     成交回报 → trades.log
        └── wal        WAL 预写日志
        └── snapshot   快照
```

单线程模式下所有环节串行。`pipeline` 模式用三个线程：主线程发命令、撮合线程处理、写出线程落盘，channel 连接。

## 数据结构

- 买单：`BTreeMap<i64, VecDeque<u64>>`，价格降序取最优
- 卖单：`BTreeMap<i64, VecDeque<u64>>`，价格升序取最优
- 订单索引：`HashMap<u64, Order>`，撤单 O(1) 定位

## 设计取舍

**价格档位用 BTreeMap。** 需要按价格有序遍历取最优价，`HashMap` 做不到；数组在价格稀疏时会浪费空间。

**同价位用 VecDeque。** 保证 FIFO，时间优先。

**撮合单线程。** 多线程并行撮合会破坏定序，成交结果不确定。并发放在撮合之外。

**快照按订单簿顺序保存。** 如果按 `HashMap` 迭代顺序保存，恢复后同价位的 FIFO 顺序会被打乱，导致后续撮合结果偏离。

**快照必须在命令执行之后保存。** 先保存再执行会把该条命令的序号算进快照，恢复时被跳过，导致状态不一致。

**WAL 和快照都要。** 只有 WAL，重启要重放全部历史；只有快照，两次快照之间的数据会丢。

## 限制

学习项目，不是生产系统。未实现：分布式部署、真实交易所协议、保证金和风控、多品种隔离、IOC/FOK 订单类型。

## 开发文档

更详细的范围、里程碑和设计说明见 [DEVELOPMENT.md](DEVELOPMENT.md)。
