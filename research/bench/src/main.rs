//! 一次性研究用基准：不是产品代码，只用来给设计文档里的性能预算提供实测锚点。
//! 运行：cargo run --release

use std::time::Instant;

fn build_doc(target_chars: usize) -> String {
    let para = "文档编辑器的核心不是把每一种格式都实现一遍，而是让同一份内容在编辑、排版、导出三个阶段保持一致。排版引擎需要知道每个字符的宽度、每个段落的间距、每一行的断点位置；编辑层需要知道光标落在哪个字符上、选区跨越了哪些块。两者之间的契约越窄，系统就越容易替换其中的一部分。这里用一段足够长的中文来模拟真实的写作内容，其中夹杂 English words and 123 numbers 以触发双向文本与字体回退路径。";
    let list_item = "- 增量解析只重算被编辑的那一块，其余块的排版结果继续复用；这是把输入延迟压在 16 毫秒以内的前提。";
    let mut s = String::with_capacity(target_chars * 3 + 4096);
    let mut i = 0usize;
    while s.chars().count() < target_chars {
        match i % 7 {
            0 => s.push_str(&format!("\n## 第 {} 节 设计与取舍\n\n", i / 7 + 1)),
            1 | 2 | 3 => {
                s.push_str(para);
                s.push_str("\n\n");
            }
            4 => {
                for _ in 0..4 {
                    s.push_str(list_item);
                    s.push('\n');
                }
                s.push('\n');
            }
            5 => {
                s.push_str("```rust\nfn layout(block: &Block) -> Height { block.measure() }\n```\n\n");
            }
            _ => {
                s.push_str("| 指标 | 目标 | 说明 |\n| --- | --- | --- |\n| 首屏 | 300 ms | 冷启动到可输入 |\n| 输入延迟 | 16 ms | p95，按键到像素 |\n\n");
            }
        }
        i += 1;
    }
    s
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn pct(mut v: Vec<f64>, p: f64) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = ((v.len() as f64 - 1.0) * p).round() as usize;
    v[idx]
}

fn bench<F: FnMut() -> usize>(mut f: F, runs: usize) -> (f64, f64, usize) {
    let mut ts = Vec::with_capacity(runs);
    let mut out = 0;
    for _ in 0..runs {
        let t = Instant::now();
        out = f();
        ts.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    (median(ts.clone()), ts.iter().cloned().fold(0.0, f64::max), out)
}

fn main() {
    let doc = build_doc(50_000);
    let chars = doc.chars().count();
    println!("== 文档 ==\n字符数: {chars}\nUTF-8 字节: {}\n", doc.len());

    // 1. 全量解析
    let (m, mx, events) = bench(
        || {
            let mut n = 0;
            for _ in pulldown_cmark::Parser::new_ext(&doc, pulldown_cmark::Options::all()) {
                n += 1;
            }
            n
        },
        9,
    );
    println!("pulldown-cmark 全量解析: 中位 {m:.2} ms, 最大 {mx:.2} ms, {events} 个事件");

    let (m, mx, _) = bench(
        || {
            let arena = comrak::Arena::new();
            let root = comrak::parse_document(&arena, &doc, &comrak::Options::default());
            std::hint::black_box(root);
            0
        },
        5,
    );
    println!("comrak 全量解析（建 AST）: 中位 {m:.2} ms, 最大 {mx:.2} ms");

    // 2. 文本缓冲：随机编辑延迟
    let mut rope = ropey::Rope::from_str(&doc);
    let mut seed = 0x9E3779B97F4A7C15u64;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut lat = Vec::with_capacity(2000);
    for _ in 0..2000 {
        let idx = (rnd() as usize) % (rope.len_chars() - 2) + 1;
        let t = Instant::now();
        rope.insert_char(idx, '字');
        lat.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        rope.remove(idx..idx + 1);
        lat.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    println!(
        "ropey 单字符插入/删除: p50 {:.4} ms, p95 {:.4} ms, p99 {:.4} ms, max {:.4} ms",
        pct(lat.clone(), 0.5),
        pct(lat.clone(), 0.95),
        pct(lat.clone(), 0.99),
        lat.iter().cloned().fold(0.0, f64::max)
    );

    // 3. 排版：整篇一个 buffer（朴素做法） vs 按块排版（拟采用）
    let mut font_system = cosmic_text::FontSystem::new();
    let metrics = cosmic_text::Metrics::new(16.0, 28.0);
    let width = 800.0;

    let t = Instant::now();
    let mut whole = cosmic_text::Buffer::new(&mut font_system, metrics);
    whole.set_size(Some(width), Some(f32::MAX));
    whole.set_text(
        &doc,
        &cosmic_text::Attrs::new(),
        cosmic_text::Shaping::Advanced,
        None,
    );
    whole.shape_until_scroll(&mut font_system, false);
    let whole_ms = t.elapsed().as_secs_f64() * 1000.0;
    let lines: usize = whole.layout_runs().count();
    println!("cosmic-text 整篇一个 buffer: {whole_ms:.2} ms, {lines} 屏内行");

    // 按空行切块，逐块排版
    let blocks: Vec<&str> = doc.split("\n\n").filter(|b| !b.trim().is_empty()).collect();
    let t = Instant::now();
    let mut block_ms = Vec::with_capacity(blocks.len());
    let mut total_lines = 0usize;
    for b in &blocks {
        let t0 = Instant::now();
        let mut buf = cosmic_text::Buffer::new(&mut font_system, metrics);
        buf.set_size(Some(width), Some(f32::MAX));
        buf.set_text(
            b,
            &cosmic_text::Attrs::new(),
            cosmic_text::Shaping::Advanced,
            None,
        );
        buf.shape_until_scroll(&mut font_system, false);
        total_lines += buf.layout_runs().count();
        block_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
    }
    let total = t.elapsed().as_secs_f64() * 1000.0;
    println!(
        "cosmic-text 按块排版: {} 块, 合计 {total:.2} ms, 中位 {:.2} ms/块, 最慢 {:.2} ms, {total_lines} 行",
        blocks.len(),
        median(block_ms.clone()),
        block_ms.iter().cloned().fold(0.0, f64::max)
    );

    // 4. 单块重排（键入时真正要付的代价）：取最慢的块重排 20 次
    if let Some((worst, _)) = blocks
        .iter()
        .zip(block_ms.iter())
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
    {
        let mut lat = Vec::new();
        for _ in 0..20 {
            let t = Instant::now();
            let mut buf = cosmic_text::Buffer::new(&mut font_system, metrics);
            buf.set_size(Some(width), Some(f32::MAX));
            buf.set_text(
                worst,
                &cosmic_text::Attrs::new(),
                cosmic_text::Shaping::Advanced,
                None,
            );
            buf.shape_until_scroll(&mut font_system, false);
            lat.push(t.elapsed().as_secs_f64() * 1000.0);
        }
        println!(
            "最慢块重排: 中位 {:.2} ms, p95 {:.2} ms（块长 {} 字符）",
            median(lat.clone()),
            pct(lat.clone(), 0.95),
            worst.chars().count()
        );
    }

    // 5. 首屏：只排到填满一屏（按行数 × 行高累加 1200px）
    let t = Instant::now();
    let mut first_screen = 0usize;
    let mut acc = 0.0f32;
    for b in &blocks {
        let mut buf = cosmic_text::Buffer::new(&mut font_system, metrics);
        buf.set_size(Some(width), Some(f32::MAX));
        buf.set_text(
            b,
            &cosmic_text::Attrs::new(),
            cosmic_text::Shaping::Advanced,
            None,
        );
        buf.shape_until_scroll(&mut font_system, false);
        acc += buf.layout_runs().count() as f32 * metrics.line_height;
        first_screen += 1;
        if acc > 1200.0 {
            break;
        }
    }
    println!(
        "首屏（填满 1200px）{first_screen} 块: {:.2} ms",
        t.elapsed().as_secs_f64() * 1000.0
    );

    // 6. 并行按块排版：每线程一个 FontSystem，块按线程数均分
    use rayon::prelude::*;
    let nthreads = rayon::current_num_threads();
    let t_init = Instant::now();
    let mut systems: Vec<cosmic_text::FontSystem> =
        (0..nthreads).map(|_| cosmic_text::FontSystem::new()).collect();
    println!(
        "{} 个 FontSystem 初始化合计: {:.2} ms（每个约 {:.0} ms，字体枚举）",
        nthreads,
        t_init.elapsed().as_secs_f64() * 1000.0,
        t_init.elapsed().as_secs_f64() * 1000.0 / nthreads as f64
    );
    let chunk = blocks.len().div_ceil(nthreads);
    let t = Instant::now();
    let n: usize = systems
        .par_iter_mut()
        .enumerate()
        .map(|(i, fs)| {
            let lo = i * chunk;
            let hi = ((i + 1) * chunk).min(blocks.len());
            let mut lines = 0;
            for b in &blocks[lo..hi] {
                let mut buf = cosmic_text::Buffer::new(&mut *fs, metrics);
                buf.set_size(Some(width), Some(f32::MAX));
                buf.set_text(
                    b,
                    &cosmic_text::Attrs::new(),
                    cosmic_text::Shaping::Advanced,
                    None,
                );
                buf.shape_until_scroll(&mut *fs, false);
                lines += buf.layout_runs().count();
            }
            lines
        })
        .sum();
    println!(
        "并行排版 {} 块: {:.2} ms, {} 行, {} 线程",
        blocks.len(),
        t.elapsed().as_secs_f64() * 1000.0,
        n,
        nthreads
    );

    println!("进程工作集: {:.1} MB", rss_mb());
}

#[cfg(windows)]
fn rss_mb() -> f64 {
    #[repr(C)]
    #[allow(non_snake_case)]
    struct ProcessMemoryCounters {
        cb: u32,
        PageFaultCount: u32,
        PeakWorkingSetSize: usize,
        WorkingSetSize: usize,
        QuotaPeakPagedPoolUsage: usize,
        QuotaPagedPoolUsage: usize,
        QuotaPeakNonPagedPoolUsage: usize,
        QuotaNonPagedPoolUsage: usize,
        PagefileUsage: usize,
        PeakPagefileUsage: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(
            process: isize,
            counters: *mut ProcessMemoryCounters,
            cb: u32,
        ) -> i32;
    }
    let mut c: ProcessMemoryCounters = unsafe { std::mem::zeroed() };
    c.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
    if ok == 0 {
        return f64::NAN;
    }
    c.WorkingSetSize as f64 / (1024.0 * 1024.0)
}

#[cfg(not(windows))]
fn rss_mb() -> f64 {
    f64::NAN
}
