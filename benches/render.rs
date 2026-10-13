//! In-process wall-time of `ma::render`. Run via `scripts/run_bench.sh`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

struct CountingAlloc;

static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOC_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

struct Sample {
    times: Vec<Duration>,
    allocs: Vec<u64>,
    bytes: Vec<u64>,
    last_len: usize,
}

struct Case {
    name: String,
    input: String,
}

fn read_dir_mmd(dir: &Path, prefix: &str) -> Vec<Case> {
    let mut cases = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return cases;
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("mmd"))
        .collect();
    paths.sort();
    for path in paths {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let input =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        cases.push(Case {
            name: format!("{prefix}/{name}"),
            input,
        });
    }
    cases
}

fn median(mut values: Vec<u64>) -> u64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[values.len() / 2]
}

fn median_duration(times: &[Duration]) -> Duration {
    let nanos: Vec<u64> = times.iter().map(|d| d.as_nanos() as u64).collect();
    Duration::from_nanos(median(nanos))
}

fn min_duration(times: &[Duration]) -> Duration {
    *times.iter().min().unwrap_or(&Duration::ZERO)
}

fn max_duration(times: &[Duration]) -> Duration {
    *times.iter().max().unwrap_or(&Duration::ZERO)
}

fn reset_allocs() {
    ALLOC_COUNT.store(0, Ordering::Relaxed);
    ALLOC_BYTES.store(0, Ordering::Relaxed);
}

fn snapshot_allocs() -> (u64, u64) {
    (
        ALLOC_COUNT.load(Ordering::Relaxed),
        ALLOC_BYTES.load(Ordering::Relaxed),
    )
}

fn render_once(input: &str) -> Result<String, String> {
    ma::render(input)
}

fn sample_case(input: &str, iterations: usize, warmup: usize) -> Sample {
    for _ in 0..warmup {
        let _ = std::hint::black_box(render_once(input).expect("warmup render"));
    }
    let mut times = Vec::with_capacity(iterations);
    let mut allocs = Vec::with_capacity(iterations);
    let mut bytes = Vec::with_capacity(iterations);
    let mut last_len = 0;
    for _ in 0..iterations {
        reset_allocs();
        let start = Instant::now();
        let output = std::hint::black_box(render_once(input).expect("timed render"));
        let elapsed = start.elapsed();
        let (count, nbytes) = snapshot_allocs();
        last_len = output.len();
        times.push(elapsed);
        allocs.push(count);
        bytes.push(nbytes);
    }
    Sample {
        times,
        allocs,
        bytes,
        last_len,
    }
}

fn fmt_duration(d: Duration) -> String {
    let ns = d.as_nanos();
    if ns >= 1_000_000_000 {
        format!("{:.3}s", d.as_secs_f64())
    } else if ns >= 1_000_000 {
        format!("{:.3}ms", d.as_secs_f64() * 1e3)
    } else if ns >= 1_000 {
        format!("{:.1}µs", d.as_secs_f64() * 1e6)
    } else {
        format!("{ns}ns")
    }
}

fn iterations_for(warmup_time: Duration) -> usize {
    if warmup_time >= Duration::from_millis(200) {
        7
    } else if warmup_time >= Duration::from_millis(20) {
        15
    } else {
        40
    }
}

fn print_sample(name: &str, sample: &Sample) {
    let med = median_duration(&sample.times);
    let min = min_duration(&sample.times);
    let max = max_duration(&sample.times);
    let alloc_med = median(sample.allocs.clone());
    let bytes_med = median(sample.bytes.clone());
    println!(
        "{:<28} n={:<3}  median={}  min={}  max={}  allocs={}  alloc_bytes={}  out_bytes={}",
        name,
        sample.times.len(),
        fmt_duration(med),
        fmt_duration(min),
        fmt_duration(max),
        alloc_med,
        bytes_med,
        sample.last_len
    );
}

fn parse_args() -> (PathBuf, Option<String>, bool) {
    let mut dir = PathBuf::from("benches/inputs");
    let mut once: Option<String> = None;
    let mut list = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dir" => {
                dir = PathBuf::from(args.next().expect("--dir needs a path"));
            }
            "--once" => {
                once = Some(args.next().expect("--once needs a case name"));
            }
            "--list" => list = true,
            "--bench" | "--exact" | "--ignored" | "--nocapture" => {}
            other if other.starts_with('-') => panic!("unknown arg: {other}"),
            _ => {}
        }
    }
    (dir, once, list)
}

fn render_all_fixtures(fixtures: &[Case]) -> Sample {
    let warmup = 1;
    let iterations = 7;
    for _ in 0..warmup {
        for case in fixtures {
            let _ = std::hint::black_box(render_once(&case.input).expect("fixture warmup"));
        }
    }
    let mut times = Vec::new();
    let mut allocs = Vec::new();
    let mut bytes = Vec::new();
    let mut last_len = 0;
    for _ in 0..iterations {
        reset_allocs();
        let start = Instant::now();
        last_len = 0;
        for case in fixtures {
            last_len +=
                std::hint::black_box(render_once(&case.input).expect("fixture render")).len();
        }
        let elapsed = start.elapsed();
        let (count, nbytes) = snapshot_allocs();
        times.push(elapsed);
        allocs.push(count);
        bytes.push(nbytes);
    }
    Sample {
        times,
        allocs,
        bytes,
        last_len,
    }
}

fn main() {
    let (dir, once, list) = parse_args();
    if !dir.join("generated").is_dir() {
        panic!(
            "missing {}; run: python3 scripts/bench_inputs.py {}",
            dir.display(),
            dir.display()
        );
    }

    let generated = read_dir_mmd(&dir.join("generated"), "generated");
    let mut fixtures = read_dir_mmd(&dir.join("fixtures"), "fixture");
    if once.is_none() {
        fixtures.retain(|case| match render_once(&case.input) {
            Ok(_) => true,
            Err(err) => {
                eprintln!("skip {}: {err}", case.name);
                false
            }
        });
    }
    if list {
        for case in generated.iter().chain(fixtures.iter()) {
            println!("{}  {} bytes", case.name, case.input.len());
        }
        println!("fixtures/ALL  {} diagrams", fixtures.len());
        return;
    }

    println!(
        "ma render bench  opt-level=release  cores={}",
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    );
    println!("dir={}", dir.display());
    println!("fixtures={} generated={}", fixtures.len(), generated.len());

    if let Some(needle) = once {
        let case = generated
            .iter()
            .chain(fixtures.iter())
            .find(|c| c.name.contains(&needle))
            .unwrap_or_else(|| panic!("no case matching {needle}"));
        let output =
            render_once(&case.input).unwrap_or_else(|e| panic!("render {}: {e}", case.name));
        std::hint::black_box(output);
        return;
    }

    if !fixtures.is_empty() {
        let sample = render_all_fixtures(&fixtures);
        print_sample("fixtures/ALL", &sample);
    }

    for case in &generated {
        let start = Instant::now();
        render_once(&case.input).unwrap_or_else(|e| panic!("render {}: {e}", case.name));
        let warmup_time = start.elapsed();
        let n = iterations_for(warmup_time);
        let sample = sample_case(&case.input, n, 1);
        print_sample(&case.name, &sample);
        print_phases(&case.name, &case.input, n);
    }
}

fn print_phases(name: &str, input: &str, iterations: usize) {
    let trimmed = input.trim_start();
    if !(trimmed.starts_with("graph") || trimmed.starts_with("flowchart")) {
        return;
    }
    let mut parse_t = Vec::new();
    let mut layout_t = Vec::new();
    let mut render_t = Vec::new();
    for _ in 0..iterations {
        let start = Instant::now();
        let diagram = ma::graph_parser::parse_graph(input).expect("parse");
        parse_t.push(start.elapsed());
        let start = Instant::now();
        let layout = ma::graph_layout::compute(&diagram).expect("layout");
        layout_t.push(start.elapsed());
        let start = Instant::now();
        let _ = std::hint::black_box(ma::graph_renderer::render(&layout));
        render_t.push(start.elapsed());
    }
    println!(
        "  {name} phases  parse={}  layout={}  draw={}",
        fmt_duration(median_duration(&parse_t)),
        fmt_duration(median_duration(&layout_t)),
        fmt_duration(median_duration(&render_t))
    );
}
