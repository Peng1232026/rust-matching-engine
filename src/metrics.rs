//! 延迟统计。
//!
//! 记录每次撮合耗时的纳秒数，最后算分位数。
//! p99 = 99% 的请求耗时都低于这个值，比平均值更能反映真实体验。

pub struct LatencyStats {
    samples: Vec<u64>,
}

impl LatencyStats {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
        }
    }

    /// 预先分配容量，避免循环里反复扩容。
    /// 知道总数时应该用这个。
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            samples: Vec::with_capacity(cap),
        }
    }

    pub fn record(&mut self, ns: u64) {
        self.samples.push(ns);
    }

    /// 计算分位数，p 取值 0.0 ~ 1.0。
    ///
    /// 这里每次调用都排序，是为了演示简单。
    /// 生产环境通常用直方图，避免排序开销。
    pub fn percentile(&mut self, p: f64) -> u64 {
        if self.samples.is_empty() {
            return 0;
        }
        self.samples.sort_unstable();
        let idx = ((self.samples.len() - 1) as f64 * p) as usize;
        self.samples[idx]
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}
