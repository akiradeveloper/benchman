//! benchman is a RAII-style benchmark tool that
//! focuses on old fashioned one-shot benchmark rather than statistical benchmark.

use colored::*;
use indexmap::{IndexMap, IndexSet};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
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
    tag_indices: IndexSet<String>,
    h: HashMap<String, BenchResult>,
}
impl ResultSet {
    fn new() -> Self {
        Self {
            tag_indices: IndexSet::new(),
            h: HashMap::new(),
        }
    }
    fn reserve_tag(&mut self, tag: String) {
        self.tag_indices.insert(tag);
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
#[derive(Clone)]
pub struct BenchMan {
    tag: Arc<String>,
    result_set: Arc<RwLock<ResultSet>>,
}
impl BenchMan {
    /// Create a benchman.
    pub fn new(tag: &str) -> Self {
        let result_set = Arc::new(RwLock::new(ResultSet::new()));
        Self {
            tag: Arc::new(tag.to_owned()),
            result_set,
        }
    }
    /// Get a stopwatch from benchman.
    pub fn get_stopwatch(&self, tag: &str) -> Stopwatch {
        self.result_set.write().unwrap().reserve_tag(tag.to_owned());
        Stopwatch::new(tag.to_owned(), self.result_set.clone())
    }
    /// Get an immutable view of the benchman.
    ///
    /// If a tag in the list isn't found in the current result, the tag is ignored.
    pub fn slice<'a>(&'a self, sw_tags: impl IntoIterator<Item = &'a str>) -> BenchManSlice<'a> {
        let result_set_reader = &self.result_set.read().unwrap();
        let mut m = IndexMap::new();
        for sw_tag in sw_tags {
            if let Some(br) = result_set_reader.h.get(sw_tag) {
                m.insert(sw_tag, br.clone());
            }
        }
        BenchManSlice {
            bm_tag: &self.tag,
            slices: m,
        }
    }
}
impl fmt::Display for BenchMan {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bench_tag = &self.tag;
        writeln!(f, "{}", bench_tag.blue())?;
        let result_set_reader = &self.result_set.read().unwrap();
        for sw_tag in &result_set_reader.tag_indices {
            if let Some(v) = result_set_reader.h.get(sw_tag) {
                let tag = format!("{} ({} samples)", sw_tag, v.n());
                writeln!(f, "{}", tag.yellow())?;
                writeln!(f, "{}", v)?;
            }
        }
        Ok(())
    }
}
/// Immutable view of the benchman.
pub struct BenchManSlice<'a> {
    bm_tag: &'a str,
    slices: IndexMap<&'a str, BenchResult>,
}
impl<'a> fmt::Display for BenchManSlice<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bench_tag = &self.bm_tag;
        writeln!(f, "{}", bench_tag.blue())?;
        for (sw_tag, br) in &self.slices {
            let tag = format!("{} ({} samples)", sw_tag, br.n());
            writeln!(f, "{}", tag.yellow())?;
            writeln!(f, "{}", br)?;
        }
        Ok(())
    }
}

/// On drop, it sends a result to the benchman.
pub struct Stopwatch {
    tag: Option<String>,
    t: Instant,
    result_set: Arc<RwLock<ResultSet>>,
}
impl Stopwatch {
    fn new(tag: String, result_set: Arc<RwLock<ResultSet>>) -> Self {
        Self {
            tag: Some(tag),
            t: Instant::now(),
            result_set,
        }
    }
}
impl Drop for Stopwatch {
    fn drop(&mut self) {
        let elapsed = self.t.elapsed();
        let sw_tag = self.tag.take().unwrap();
        self.result_set.write().unwrap().add_result(sw_tag, elapsed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchman_spawn() {
        let benchman = BenchMan::new("spawn");
        for _ in 0..1 {
            let bm = benchman.clone();
            std::thread::spawn(move || {
                let _sw = bm.get_stopwatch("loop1");
                let mut _sum: u64 = 0;
                for i in 0..1000000 {
                    _sum += i;
                }
            });
        }
        for _ in 0..100 {
            let bm = benchman.clone();
            std::thread::spawn(move || {
                let _sw = bm.get_stopwatch("loop2");
                let mut _sum: u64 = 0;
                for i in 0..1000000 {
                    _sum += i;
                }
            });
        }
        std::thread::sleep(Duration::from_secs(1));
        println!("{}", benchman);
    }

    #[test]
    fn test_benchman_nested() {
        let benchman = BenchMan::new("nested");
        let mut _sum: u64 = 0;
        let sw1 = benchman.get_stopwatch("outer");
        for i in 0..1000 {
            let _sw2 = benchman.get_stopwatch("inner");
            for j in 0..100000 {
                _sum += i * j;
            }
        }
        println!("{}", benchman.slice(["inner", "outer"]));
        drop(sw1);
        println!("{}", benchman.slice(["inner", "outer"]));
        println!("{}", benchman);
    }
}
