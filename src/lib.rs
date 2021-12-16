//! benchman is a RAII-style benchmark tool that
//! focuses on old fashioned one-shot benchmark rather than statistical benchmark.

use colored::*;
use std::collections::HashMap;
use std::fmt;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct BenchResult {
    list: Vec<Duration>,
}
impl BenchResult {
    fn new() -> Self {
        Self { list: vec![] }
    }
    fn n(&self) -> usize {
        self.list.len()
    }
    fn add_result(&mut self, du: Duration) {
        self.list.push(du);
    }
    fn average(&self) -> Duration {
        let n = self.list.len();
        let mut sum = Duration::from_secs(0);
        for &du in &self.list {
            sum += du;
        }
        sum / (n as u32)
    }
    fn percentile(&self, p: u64) -> Duration {
        assert!(p > 0);
        let mut list = self.list.clone();
        list.sort();
        let p = p as f64 / 100.;
        let n = self.list.len() as f64;
        let i = f64::ceil(p * n) as usize;
        list[i - 1]
    }
}
impl fmt::Display for BenchResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let n = self.list.len();
        let p50 = self.percentile(50);
        let p95 = self.percentile(95);
        let p99 = self.percentile(99);
        writeln!(f, "[ave.] {:?}", self.average())?;
        writeln!(f, "{:?} (>50%), {:?} (>95%), {:?} (>99%)", p50, p95, p99)?;
        Ok(())
    }
}
#[derive(Debug)]
struct ResultSet {
    pub h: HashMap<String, BenchResult>,
}
impl ResultSet {
    fn new() -> Self {
        Self { h: HashMap::new() }
    }
    fn add_result(&mut self, tag: String, du: Duration) {
        self.h
            .entry(tag)
            .or_insert(BenchResult::new())
            .add_result(du);
    }
}
/// Benchman who collects the result from stopwatches.
///
/// ```rust
/// use benchman::*;
/// let bm = BenchMan::new("bm_tag");
/// let sw = bm.get_stopwatch("sw_tag");
/// let mut sum = 0;
/// for i in 1..10 { sum += i; }
/// drop(sw);
/// eprintln!("{}", bm);
/// ```
pub struct BenchMan {
    tag: String,
    tx: mpsc::Sender<Msg>,
    result_set: Arc<RwLock<ResultSet>>,
}
struct Msg(String, Duration);
impl BenchMan {
    /// Create a benchman.
    pub fn new(tag: &str) -> Self {
        let (tx, mut rx) = mpsc::channel();
        let result_set = Arc::new(RwLock::new(ResultSet::new()));
        let result_set_cln = result_set.clone();
        std::thread::spawn(move || {
            while let Ok(Msg(tag, du)) = rx.recv() {
                result_set_cln.write().unwrap().add_result(tag, du);
            }
        });
        Self {
            tag: tag.to_owned(),
            tx,
            result_set,
        }
    }
    /// Get a stopwatch from benchman.
    pub fn get_stopwatch(&self, tag: &str) -> Stopwatch {
        Stopwatch::new(tag.to_owned(), self.tx.clone())
    }
}
impl fmt::Display for BenchMan {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bench_tag = &self.tag;
        writeln!(f, "{}", bench_tag.blue())?;
        // This sleep is to wait for the in-flight messasges.
        std::thread::sleep(Duration::from_secs(1));
        for (sw_tag, v) in &self.result_set.read().unwrap().h {
            let tag = format!("{} ({} samples)", sw_tag, v.n());
            writeln!(f, "{}", tag.yellow())?;
            writeln!(f, "{}", v)?;
        }
        Ok(())
    }
}

/// On drop, it sends a result to the benchman.
pub struct Stopwatch {
    tag: Option<String>,
    t: Instant,
    tx: mpsc::Sender<Msg>,
}
impl Stopwatch {
    fn new(tag: String, tx: mpsc::Sender<Msg>) -> Self {
        Self {
            tag: Some(tag),
            tx,
            t: Instant::now(),
        }
    }
}
impl Drop for Stopwatch {
    fn drop(&mut self) {
        let elapsed = self.t.elapsed();
        self.tx.send(Msg(self.tag.take().unwrap(), elapsed)).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchman_spawn() {
        let benchman = BenchMan::new("spawn");
        for _ in 0..1 {
            let stopwatch = benchman.get_stopwatch("loop1");
            std::thread::spawn(move || {
                let mut sum: u64 = 0;
                for i in 0..1000000 {
                    sum += i;
                }
                drop(stopwatch);
            });
        }
        for _ in 0..100 {
            let stopwatch = benchman.get_stopwatch("loop2");
            std::thread::spawn(move || {
                let mut sum: u64 = 0;
                for i in 0..1000000 {
                    sum += i;
                }
                drop(stopwatch);
            });
        }
        println!("{}", benchman);
    }

    #[test]
    fn test_benchman_nested() {
        let benchman = BenchMan::new("nested");
        let mut sum: u64 = 0;
        let s1 = benchman.get_stopwatch("outer");
        for i in 0..1000 {
            let s2 = benchman.get_stopwatch("inner");
            for j in 0..100000 {
                sum += i * j;
            }
        }
        drop(s1);
        println!("{}", benchman);
    }
}
