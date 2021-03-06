# benchman

[![Crates.io](https://img.shields.io/crates/v/benchman.svg)](https://crates.io/crates/benchman)
[![documentation](https://docs.rs/benchman/badge.svg)](https://docs.rs/benchman)

![2f5c999cf5f782f8d5ec224c9149aca9899849c6_l](https://user-images.githubusercontent.com/785824/146725336-11857d72-b1fe-4e39-ae3c-6c071e82f367.jpeg)

## Features

- Focus on one-shot benchmark
- RAII-style
- Statistics (Average, Median, 95% and 99% percentile)
- Colored output
- Tagging
- Nesting

## Motivation

I guess there are two types of benchmarks.

One is a benchmark of a small and fast function in which we want the statistics
from a million of iterations. For this type of benchmark, [Criterion.rs](https://github.com/bheisler/criterion.rs) is a good fit.

Another type is what I call one-shot benchmark.

You may have wanted to write a benchmark program like this.

```rust
let mut db = DB::new();

let t = Instant::now();
db.write(...);
println!("write: {:?}", t.elapsed());

let t = Instant::now();
db.read(...);
println!("read: {:?}", t.elapsed());
```

According to [Criterion.rs #531](https://github.com/bheisler/criterion.rs/issues/531), this type of benchmark is infeasible with Criterion.rs because Criterion is focusing on the first type.

That's why I started to create benchman.

## RAII-style measurement

RAII is a good technique to manage resource access.
My idea behind designing benchman is that stopwatch is like a resource
because it is like a producer of a benchmark result that sends the result to the
single central consumer and there is a strict rule that stopwatch shouldn't send the result twice.

With this idea, the library is designed like this.

```rust
let stopwatch = benchman.get_stopwatch("some_tag");
do_something();
drop(stopwatch);

// or

{
    let _sw = benchman.get_stopwatch("some_tag");
    do_something();
}
```

When the stopwatch is dropped, the measurement result is sent to the central database.

## Screenshot

![スクリーンショット 2021-12-18 12 42 40](https://user-images.githubusercontent.com/785824/146627751-8df2ffad-872c-4421-8b75-7518d0505a4d.png)

## Author

Akira Hayakawa (@akiradeveloper)
