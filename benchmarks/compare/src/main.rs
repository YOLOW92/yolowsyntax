use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::Instant;
use yolowsyntax::Limits;
use yolowsyntax::api::parse;
use yolowsyntax::corpus::{Case, Report, parse_corpus, run_cases, strip_fields};
use yolowsyntax::langs::Lang;
use yolowsyntax::outline;

struct Spec {
    tag: &'static str,
    corpus_subpath: &'static str,
    ts_lang: fn() -> tree_sitter::Language,
}

const SPECS: &[Spec] = &[
    Spec {
        tag: "c",
        corpus_subpath: "c/test/corpus",
        ts_lang: || tree_sitter_c::LANGUAGE.into(),
    },
    Spec {
        tag: "go",
        corpus_subpath: "go/test/corpus",
        ts_lang: || tree_sitter_go::LANGUAGE.into(),
    },
    Spec {
        tag: "java",
        corpus_subpath: "java/test/corpus",
        ts_lang: || tree_sitter_java::LANGUAGE.into(),
    },
    Spec {
        tag: "python",
        corpus_subpath: "python/test/corpus",
        ts_lang: || tree_sitter_python::LANGUAGE.into(),
    },
    Spec {
        tag: "javascript",
        corpus_subpath: "javascript/test/corpus",
        ts_lang: || tree_sitter_javascript::LANGUAGE.into(),
    },
    Spec {
        tag: "typescript",
        corpus_subpath: "typescript/test/corpus",
        ts_lang: || tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
    },
    Spec {
        tag: "rust",
        corpus_subpath: "rust/test/corpus",
        ts_lang: || tree_sitter_rust::LANGUAGE.into(),
    },
    Spec {
        tag: "bash",
        corpus_subpath: "bash/test/corpus",
        ts_lang: || tree_sitter_bash::LANGUAGE.into(),
    },
];

fn collect_txt(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return files;
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if p.is_dir() {
            files.extend(collect_txt(&p));
        } else if p.is_file()
            && p.extension()
                .and_then(|s| s.to_str())
                .is_some_and(|e| matches!(e, "txt" | "test" | "mk" | "scm"))
        {
            files.push(p);
        }
    }
    files.sort();
    files
}

const ALL_TAGS: &[&str] = &[
    "c",
    "cpp",
    "csharp",
    "go",
    "java",
    "javascript",
    "typescript",
    "tsx",
    "php",
    "python",
    "rust",
    "lua",
    "bash",
    "ruby",
    "kotlin",
    "css",
    "html",
    "objc",
    "dart",
    "julia",
    "nix",
    "dockerfile",
    "ocaml",
    "elixir",
    "r",
    "svelte",
    "solidity",
    "groovy",
    "make",
    "scheme",
    "commonlisp",
    "pascal",
    "gleam",
    "powershell",
    "cmake",
    "erlang",
    "hcl",
    "vue",
    "d",
    "elm",
    "verilog",
];

fn find_corpus_dir(corpus_root: &Path, tag: &str) -> PathBuf {
    let base = corpus_root.join(tag);
    let tc = base.join("test/corpus");
    if tc.is_dir() {
        return tc;
    }
    let c = base.join("corpus");
    if c.is_dir() {
        return c;
    }
    base
}

fn load_cases_for_dir(dir: &Path) -> Vec<(String, Vec<Case>)> {
    collect_txt(dir)
        .into_iter()
        .map(|p| {
            let name = p
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let raw = fs::read(&p).expect("read corpus");
            let text = String::from_utf8_lossy(&raw).into_owned();
            (name, parse_corpus(&text))
        })
        .collect()
}

fn load_cases(corpus_root: &Path, spec: &Spec) -> Vec<(String, Vec<Case>)> {
    load_cases_for_dir(&corpus_root.join(spec.corpus_subpath))
}

fn run_ts_cases(
    ts_lang: &tree_sitter::Language,
    lang_name: &str,
    cases: &[Case],
) -> (usize, usize, usize) {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(ts_lang).expect("set_language");
    let (mut total, mut passed, mut skipped) = (0, 0, 0);
    for c in cases {
        if c.skip
            || c.cst
            || !c.platform_ok
            || c.language.as_deref().is_some_and(|l| l != lang_name)
        {
            skipped += 1;
            continue;
        }
        total += 1;
        let Some(tree) = parser.parse(&c.input, None) else {
            continue;
        };
        let root = tree.root_node();
        let ok = if c.error {
            root.has_error()
        } else {
            let has_fields = c.expected.contains(": (");
            let s = root.to_sexp();
            let actual = if has_fields { s } else { strip_fields(&s) };
            actual == c.expected
        };
        if ok {
            passed += 1;
        }
    }
    (total, passed, skipped)
}

fn cmd_corpus(corpus_root: &Path) {
    println!("lang\tcases\tyolow_pass\tyolow_pct\tts_pass\tts_pct\tskipped");
    for spec in SPECS {
        let lang = Lang::from_tag(spec.tag).unwrap().table().unwrap();
        let ts_lang = (spec.ts_lang)();
        let files = load_cases(corpus_root, spec);
        let mut y_rep = Report::default();
        let (mut ts_total, mut ts_passed, mut _ts_skip) = (0, 0, 0);
        for (fname, cases) in &files {
            run_cases(lang, fname, cases, &mut y_rep);
            let (t, p, s) = run_ts_cases(&ts_lang, &lang.name, cases);
            ts_total += t;
            ts_passed += p;
            _ts_skip += s;
        }
        let y_pct = if y_rep.total > 0 {
            100.0 * y_rep.passed as f64 / y_rep.total as f64
        } else {
            0.0
        };
        let ts_pct = if ts_total > 0 {
            100.0 * ts_passed as f64 / ts_total as f64
        } else {
            0.0
        };
        println!(
            "{}\t{}\t{}\t{:.2}%\t{}\t{:.2}%\t{}",
            spec.tag, y_rep.total, y_rep.passed, y_pct, ts_passed, ts_pct, y_rep.skipped
        );
    }
}

fn build_payload_from_files(files: &[(String, Vec<Case>)], target_bytes: usize) -> String {
    let mut valid_snips: Vec<String> = Vec::new();
    for (_, cases) in files {
        for c in cases {
            if !c.skip && !c.error && !c.cst && c.platform_ok && c.language.is_none() {
                if let Ok(s) = std::str::from_utf8(&c.input) {
                    if !s.trim().is_empty() {
                        valid_snips.push(s.to_string());
                    }
                }
            }
        }
    }
    if valid_snips.is_empty() {
        for (_, cases) in files {
            for c in cases {
                if !c.skip && !c.error && !c.cst && c.platform_ok {
                    if let Ok(s) = std::str::from_utf8(&c.input) {
                        if !s.trim().is_empty() {
                            valid_snips.push(s.to_string());
                        }
                    }
                }
            }
        }
    }
    let mut out = String::with_capacity(target_bytes + 4096);
    if valid_snips.is_empty() {
        return out;
    }
    let mut idx = 0;
    while out.len() < target_bytes {
        out.push_str(&valid_snips[idx % valid_snips.len()]);
        out.push('\n');
        idx += 1;
    }
    out
}

fn build_payload(corpus_root: &Path, spec: &Spec, target_bytes: usize) -> String {
    let files = load_cases(corpus_root, spec);
    build_payload_from_files(&files, target_bytes)
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn verify_pair(
    tag: &str,
    payload: &str,
    lang: &yolowsyntax::table::Language,
    ts_parser: &mut tree_sitter::Parser,
) -> (&'static str, String) {
    let bytes = payload.as_bytes();
    let y_tree = parse(lang, bytes, Limits::default());
    assert!(!y_tree.truncated, "{tag}: yolowsyntax returned truncated");
    assert_ne!(y_tree.root, yolowsyntax::tree::NONE, "{tag}: NONE root");
    let ts_tree = ts_parser
        .parse(bytes, None)
        .expect("ts parse returned None");
    let y_err = y_tree.has_error();
    let ts_err = ts_tree.root_node().has_error();
    let y_sexp = strip_fields(&y_tree.to_sexp(lang));
    let ts_sexp = strip_fields(&ts_tree.root_node().to_sexp());
    assert_eq!(
        y_err, ts_err,
        "{tag}: error parity mismatch (y_err={y_err}, ts_err={ts_err})"
    );
    let parity = if y_sexp == ts_sexp {
        if y_err { "exact(both_err)" } else { "exact" }
    } else if !y_err {
        if tag == "python" {
            "clean(py_expr_stmt)"
        } else {
            "clean_diff"
        }
    } else {
        "both_err_diff"
    };
    let out_status = match outline(tag, payload) {
        Some(o) => {
            let rebuilt: String = o
                .sections
                .iter()
                .map(|s| &payload[s.start..s.end])
                .collect();
            assert_eq!(rebuilt, payload, "{tag}: outline sections not lossless");
            format!("some({})", o.symbols.len())
        }
        None => "none".to_string(),
    };
    (parity, out_status)
}

fn bench_payload(
    tag: &str,
    payload: &str,
    cold_ms: f64,
    lang: &yolowsyntax::table::Language,
    ts_parser: &mut tree_sitter::Parser,
    rounds: usize,
) {
    let bytes = payload.as_bytes();
    let (parity, out_status) = verify_pair(tag, payload, lang, ts_parser);

    // Warmup
    black_box(parse(lang, bytes, Limits::default()));
    black_box(ts_parser.parse(bytes, None));
    black_box(outline(tag, payload));

    let mut y_parse_times = Vec::with_capacity(rounds);
    let mut ts_parse_times = Vec::with_capacity(rounds);
    let mut y_outline_times = Vec::with_capacity(rounds);

    for r in 0..rounds {
        if r % 2 == 0 {
            let t0 = Instant::now();
            let t = black_box(parse(lang, bytes, Limits::default()));
            assert!(!t.truncated);
            y_parse_times.push(t0.elapsed().as_secs_f64() * 1e3);

            let t1 = Instant::now();
            black_box(ts_parser.parse(bytes, None));
            ts_parse_times.push(t1.elapsed().as_secs_f64() * 1e3);
        } else {
            let t1 = Instant::now();
            black_box(ts_parser.parse(bytes, None));
            ts_parse_times.push(t1.elapsed().as_secs_f64() * 1e3);

            let t0 = Instant::now();
            let t = black_box(parse(lang, bytes, Limits::default()));
            assert!(!t.truncated);
            y_parse_times.push(t0.elapsed().as_secs_f64() * 1e3);
        }
        let t2 = Instant::now();
        black_box(outline(tag, payload));
        y_outline_times.push(t2.elapsed().as_secs_f64() * 1e3);
    }

    let yp = median(&mut y_parse_times);
    let tp = median(&mut ts_parse_times);
    let yo = median(&mut y_outline_times);
    let mb = bytes.len() as f64 / (1024.0 * 1024.0);
    let y_mbs = mb / (yp / 1e3);
    let ts_mbs = mb / (tp / 1e3);
    let ratio = tp / yp;

    println!(
        "{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.2}\t{:.2}\t{:.2}x\t{}\t{}",
        tag,
        bytes.len(),
        cold_ms,
        yp,
        tp,
        yo,
        y_mbs,
        ts_mbs,
        ratio,
        parity,
        out_status
    );
}

fn cmd_bench(corpus_root: &Path, target_kb: usize, rounds: usize) {
    let target_bytes = target_kb * 1024;
    println!(
        "lang\tbytes\tcold_load_ms\tyolow_parse_ms\tts_parse_ms\tyolow_outline_ms\tyolow_mb_s\tts_mb_s\tratio_ts_over_yolow\tparity\toutline"
    );
    for spec in SPECS {
        let payload = build_payload(corpus_root, spec, target_bytes);
        let cold_t0 = Instant::now();
        let lang = Lang::from_tag(spec.tag).unwrap().table().unwrap();
        let cold_ms = cold_t0.elapsed().as_secs_f64() * 1e3;

        let ts_lang = (spec.ts_lang)();
        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser.set_language(&ts_lang).unwrap();
        bench_payload(spec.tag, &payload, cold_ms, lang, &mut ts_parser, rounds);
    }
}

fn cmd_real_bench(corpus_root: &Path, rounds: usize) {
    let real_files = [
        ("c", "c.c"),
        ("go", "go.go"),
        ("java", "java.java"),
        ("python", "python.py"),
        ("javascript", "javascript.js"),
        ("typescript", "typescript.ts"),
        ("rust", "rust.rs"),
        ("bash", "bash.sh"),
    ];
    println!(
        "lang\tbytes\tcold_load_ms\tyolow_parse_ms\tts_parse_ms\tyolow_outline_ms\tyolow_mb_s\tts_mb_s\tratio_ts_over_yolow\tparity\toutline"
    );
    for (spec, (_, fname)) in SPECS.iter().zip(real_files.iter()) {
        let path = corpus_root.join("_real").join(fname);
        let payload = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let cold_t0 = Instant::now();
        let lang = Lang::from_tag(spec.tag).unwrap().table().unwrap();
        let cold_ms = cold_t0.elapsed().as_secs_f64() * 1e3;

        let ts_lang = (spec.ts_lang)();
        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser.set_language(&ts_lang).unwrap();
        bench_payload(spec.tag, &payload, cold_ms, lang, &mut ts_parser, rounds);
    }
}

fn vm_hwm_kib() -> Option<usize> {
    let s = fs::read_to_string("/proc/self/status").ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let num = rest.split_whitespace().next()?;
            return num.parse().ok();
        }
    }
    None
}

fn cmd_all_corpus(corpus_root: &Path) {
    println!("lang\tcases\tyolow_pass\tyolow_pct\tskipped");
    let (mut tot_cases, mut tot_pass, mut tot_skip) = (0usize, 0usize, 0usize);
    for &tag in ALL_TAGS {
        let lang = Lang::from_tag(tag).unwrap().table().unwrap();
        let dir = find_corpus_dir(corpus_root, tag);
        let files = load_cases_for_dir(&dir);
        let mut rep = Report::default();
        for (fname, cases) in &files {
            run_cases(lang, fname, cases, &mut rep);
        }
        tot_cases += rep.total;
        tot_pass += rep.passed;
        tot_skip += rep.skipped;
        let pct = if rep.total > 0 {
            100.0 * rep.passed as f64 / rep.total as f64
        } else {
            0.0
        };
        println!(
            "{}\t{}\t{}\t{:.2}%\t{}",
            tag, rep.total, rep.passed, pct, rep.skipped
        );
        if !rep.failures.is_empty() {
            for (name, _, _) in rep.failures.iter().take(3) {
                eprintln!("  [fail] {tag}: {name}");
            }
        }
    }
    let tot_pct = if tot_cases > 0 {
        100.0 * tot_pass as f64 / tot_cases as f64
    } else {
        0.0
    };
    println!(
        "TOTAL\t{}\t{}\t{:.2}%\t{}",
        tot_cases, tot_pass, tot_pct, tot_skip
    );
}

fn cmd_all_bench(corpus_root: &Path, target_kb: usize, rounds: usize) {
    let target_bytes = target_kb * 1024;
    println!(
        "lang\tbytes\tcold_load_ms\tyolow_parse_ms\tyolow_outline_ms\tyolow_mb_s\thas_error\toutline"
    );
    for &tag in ALL_TAGS {
        let dir = find_corpus_dir(corpus_root, tag);
        let files = load_cases_for_dir(&dir);
        let payload = build_payload_from_files(&files, target_bytes);
        let bytes = payload.as_bytes();
        let cold_t0 = Instant::now();
        let lang = Lang::from_tag(tag).unwrap().table().unwrap();
        let cold_ms = cold_t0.elapsed().as_secs_f64() * 1e3;
        if bytes.is_empty() {
            println!(
                "{}\t0\t{:.3}\t0.000\t0.000\t0.00\tfalse\tnone",
                tag, cold_ms
            );
            continue;
        }
        let y_check = parse(lang, bytes, Limits::default());
        assert!(!y_check.truncated, "{tag}: yolowsyntax returned truncated");
        let has_err = y_check.has_error();
        let out_status = match outline(tag, &payload) {
            Some(o) => {
                let rebuilt: String = o
                    .sections
                    .iter()
                    .map(|s| &payload[s.start..s.end])
                    .collect();
                assert_eq!(rebuilt, payload, "{tag}: outline sections not lossless");
                format!("some({})", o.symbols.len())
            }
            None => "none".to_string(),
        };
        black_box(y_check);

        let mut parse_times = Vec::with_capacity(rounds);
        let mut outline_times = Vec::with_capacity(rounds);
        for _ in 0..rounds {
            let t0 = Instant::now();
            let t = black_box(parse(lang, bytes, Limits::default()));
            assert!(!t.truncated);
            parse_times.push(t0.elapsed().as_secs_f64() * 1e3);

            let t1 = Instant::now();
            black_box(outline(tag, &payload));
            outline_times.push(t1.elapsed().as_secs_f64() * 1e3);
        }
        let yp = median(&mut parse_times);
        let yo = median(&mut outline_times);
        let mb = bytes.len() as f64 / (1024.0 * 1024.0);
        let mbs = mb / (yp / 1e3);
        println!(
            "{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.2}\t{}\t{}",
            tag,
            bytes.len(),
            cold_ms,
            yp,
            yo,
            mbs,
            has_err,
            out_status
        );
    }
}

fn cmd_rss(corpus_root: &Path, engine: &str, tag: &str, target_kb: usize) {
    let dir = find_corpus_dir(corpus_root, tag);
    let files = load_cases_for_dir(&dir);
    let payload = build_payload_from_files(&files, target_kb * 1024);
    let bytes = payload.as_bytes();
    match engine {
        "yolow" => {
            let lang = Lang::from_tag(tag).expect("unknown lang").table().unwrap();
            for _ in 0..5 {
                black_box(parse(lang, bytes, Limits::default()));
            }
        }
        "ts" => {
            let spec = SPECS
                .iter()
                .find(|s| s.tag == tag)
                .expect("unknown ts lang");
            let ts_lang = (spec.ts_lang)();
            let mut p = tree_sitter::Parser::new();
            p.set_language(&ts_lang).unwrap();
            for _ in 0..5 {
                black_box(p.parse(bytes, None));
            }
        }
        _ => panic!("engine must be yolow or ts"),
    }
    println!("{}\t{}\t{}", tag, engine, vm_hwm_kib().unwrap_or(0));
}

fn main() {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "corpus".to_string());
    let corpus_root = PathBuf::from(args.next().unwrap_or_else(|| "../corpus".to_string()));
    match cmd.as_str() {
        "corpus" => cmd_corpus(&corpus_root),
        "all-corpus" => cmd_all_corpus(&corpus_root),
        "bench" => {
            let kb: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(64);
            let rounds: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(15);
            cmd_bench(&corpus_root, kb, rounds);
        }
        "real-bench" => {
            let rounds: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(25);
            cmd_real_bench(&corpus_root, rounds);
        }
        "all-bench" => {
            let kb: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(64);
            let rounds: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(15);
            cmd_all_bench(&corpus_root, kb, rounds);
        }
        "rss" => {
            let engine = args.next().expect("engine");
            let tag = args.next().expect("tag");
            let kb: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(64);
            cmd_rss(&corpus_root, &engine, &tag, kb);
        }
        _ => panic!("unknown cmd: {cmd}"),
    }
}
