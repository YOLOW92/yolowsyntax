//! 第 0.1.2 轮整改回归与边界测试：资源限制、协作式超时、表验证、往返完整性与内存整理。

use yolowsyntax::Limits;
use yolowsyntax::api::parse;
use yolowsyntax::langs::Lang;
use yolowsyntax::parser::Parser;
use yolowsyntax::table::{Action, Language, NO_SYM};
use yolowsyntax::{outline, tree::NONE};

const ALL_TAGS: [&str; 41] = [
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

fn json_bytes() -> &'static [u8] {
    include_bytes!("data/json.pslg")
}

#[test]
fn all_41_runtime_tables_roundtrip_via_to_bytes() {
    for tag in ALL_TAGS {
        let lang = Lang::from_tag(tag).expect("known tag");
        let rt = lang.table().expect("runtime table loads");
        let raw = rt.to_bytes();
        let reloaded =
            Language::from_bytes(&raw).unwrap_or_else(|e| panic!("{tag} roundtrip failed: {e}"));
        let sample = match tag {
            "python" => b"def f(x):\n    return x + 1\n".as_slice(),
            "rust" => b"fn main() { let x = 1; }\n".as_slice(),
            "c" | "cpp" => b"int f(int x) { return x + 1; }\n".as_slice(),
            "html" | "svelte" | "vue" => b"<div>hi</div>\n".as_slice(),
            _ => b"x = 1\n".as_slice(),
        };
        let t_rt = parse(rt, sample, Limits::default());
        let t_re = parse(&reloaded, sample, Limits::default());
        assert!(!t_rt.truncated, "{tag} rt truncated");
        assert!(!t_re.truncated, "{tag} reloaded truncated");
        assert_eq!(
            t_rt.to_sexp(rt),
            t_re.to_sexp(&reloaded),
            "{tag} sexp mismatch after runtime to_bytes roundtrip",
        );
    }
}

#[test]
fn malformed_tables_fail_validation_without_panic_or_oom() {
    let base = Language::from_bytes(json_bytes()).unwrap();

    // 1. 错误魔数与截断
    assert!(Language::from_bytes(b"BADMAGIC\0").is_err());
    for len in [
        0usize,
        4,
        8,
        24,
        json_bytes().len() / 2,
        json_bytes().len() - 1,
    ] {
        assert!(
            Language::from_bytes(&json_bytes()[..len]).is_err(),
            "truncated len={len} should fail"
        );
    }

    // 2. 尾部多余字节
    let mut extra = json_bytes().to_vec();
    extra.extend_from_slice(&[0, 1, 2, 3]);
    assert!(Language::from_bytes(&extra).is_err());

    // 3. strs() 巨大长度字段（防止 0..u32::MAX collect 触发巨额 prealloc OOM）
    let mut oom_str = b"PSLG2\0\0\0".to_vec();
    oom_str.extend_from_slice(&4u32.to_le_bytes());
    oom_str.extend_from_slice(b"json");
    oom_str.extend_from_slice(&[0u8; 44]); // 11 u32 header fields
    oom_str.extend_from_slice(&u32::MAX.to_le_bytes()); // symbol_names len = u32::MAX
    assert!(Language::from_bytes(&oom_str).is_err());

    // 4. state_start 非单调
    let mut l = base.clone();
    l.state_start.swap(1, 2);
    assert!(Language::from_bytes(&l.to_bytes()).is_err());

    // 5. entries 非终结符 Goto 越界
    let mut l = base.clone();
    if let Some(ent) = l.entries.iter_mut().find(|e| (**e >> 16) >= l.token_count) {
        *ent = (*ent & 0xFFFF_0000) | l.state_count;
    }
    assert!(Language::from_bytes(&l.to_bytes()).is_err());

    // 6. entries 终结符指向非 Header 动作下标
    let mut l = base.clone();
    if let Some(ent) = l
        .entries
        .iter_mut()
        .find(|e| (**e >> 16) < l.token_count && (**e & 0xFFFF) > 0)
    {
        *ent = (*ent & 0xFFFF_0000) | (l.actions.len() as u32 + 5);
    }
    assert!(Language::from_bytes(&l.to_bytes()).is_err());

    // 7. entries 同一状态内符号乱序
    let mut l = base.clone();
    for s in 0..l.state_count as usize {
        let (a, b) = (l.state_start[s] as usize, l.state_start[s + 1] as usize);
        if b - a >= 2 {
            l.entries.swap(a, a + 1);
            break;
        }
    }
    assert!(Language::from_bytes(&l.to_bytes()).is_err());

    // 8. Action::Shift 目标状态越界
    let mut l = base.clone();
    for act in &mut l.actions {
        if let Action::Shift {
            state,
            extra: false,
            ..
        } = act
        {
            *state = l.state_count as u16;
            break;
        }
    }
    assert!(Language::from_bytes(&l.to_bytes()).is_err());

    // 9. Action::Reduce 符号/prod 越界
    let mut l = base.clone();
    for act in &mut l.actions {
        if let Action::Reduce { prod, .. } = act {
            *prod = u16::MAX;
            break;
        }
    }
    assert!(Language::from_bytes(&l.to_bytes()).is_err());

    // 10. lex.starts 非单调
    let mut l = base.clone();
    if l.lex.starts.len() >= 3 {
        l.lex.starts[1] = l.lex.starts[2] + 1;
    }
    assert!(Language::from_bytes(&l.to_bytes()).is_err());

    // 11. lex.segs 转换目标越界
    let mut l = base.clone();
    let bad_target = l.lex.state_count() as u16 + 1;
    if let Some(seg) = l.lex.segs.iter_mut().find(|s| s.kind() != 0) {
        seg.packed = ((seg.kind() as u16) << 14) | (bad_target & 0x3FFF);
    }
    assert!(Language::from_bytes(&l.to_bytes()).is_err());

    // 12. lex.segs accept 越界
    let mut l = base.clone();
    if let Some(seg) = l.lex.segs.iter_mut().find(|s| s.accept != NO_SYM) {
        seg.accept = l.token_count as u16 + 10;
    }
    assert!(Language::from_bytes(&l.to_bytes()).is_err());

    // 13. lex_modes 越界
    let mut l = base.clone();
    l.lex_modes[0] = l.lex.state_count() as u16;
    assert!(Language::from_bytes(&l.to_bytes()).is_err());

    // 14. 外部扫描器字段不一致（取带扫描器的表）
    let mut py = Lang::Python.table().unwrap().clone();
    py.ext_valid.pop();
    assert!(Language::from_bytes(&py.to_bytes()).is_err());
}

#[test]
fn tiny_budgets_enforce_limits_and_shrink_degraded_arena() {
    let rust = Lang::Rust.table().unwrap();
    let sample = b"fn main() { let a = 1 + 2 * 3; }\n";

    // max_nodes == 0 / max_versions == 0 不预分配直接退化
    for lim in [
        Limits {
            max_nodes: 0,
            ..Limits::default()
        },
        Limits {
            max_versions: 0,
            ..Limits::default()
        },
    ] {
        let p = Parser::new(rust, sample, lim);
        assert!(p.aborted);
        assert_eq!(p.a.nodes.capacity(), 0);
        let tree = parse(rust, sample, lim);
        assert!(tree.truncated);
        assert!(tree.has_error());
        assert_eq!(tree.arena.nodes.len(), 1);
        assert!(tree.arena.kids.is_empty());
        assert_eq!(tree.to_sexp(rust), "(UNEXPECTED '\\0')");
    }

    // max_steps == 1 / max_nodes == 4 截断后丢弃旧 Arena
    let large_src = "fn f() { let x = 1; }\n".repeat(500);
    for lim in [
        Limits {
            max_steps: 5,
            ..Limits::default()
        },
        Limits {
            max_nodes: 4,
            ..Limits::default()
        },
        Limits {
            max_stack_depth: 2,
            ..Limits::default()
        },
    ] {
        let tree = parse(rust, large_src.as_bytes(), lim);
        assert!(tree.truncated);
        assert!(tree.has_error());
        assert_eq!(tree.arena.nodes.len(), 1);
        assert!(tree.arena.kids.is_empty());
        assert!(tree.arena.ext.is_empty());
        assert_eq!(tree.to_sexp(rust), "(UNEXPECTED '\\0')");
    }

    // max_versions 生效：不同配置下在歧义 C++ 代码上可解析且严格受限
    let cpp = Lang::Cpp.table().unwrap();
    let ambig = b"void f() { A * b; C<D<E>> x; }\n";
    let t1 = parse(
        cpp,
        ambig,
        Limits {
            max_versions: 1,
            ..Limits::default()
        },
    );
    let t6 = parse(cpp, ambig, Limits::default());
    assert!(!t1.truncated && !t6.truncated);
    assert!(!t6.to_sexp(cpp).is_empty());

    // 预分配有合理上限，不会对大输入一次预留数百万节点
    let fake_large = vec![b' '; 4 << 20];
    let p = Parser::new(rust, &fake_large, Limits::default());
    assert!(
        p.a.nodes.capacity() <= 32_768,
        "unexpected prealloc cap={}",
        p.a.nodes.capacity()
    );
}

#[test]
fn cooperative_timeout_stops_lexer_and_external_scanners_quickly() {
    // 1. 纯长注释 / 空白：验证 Lexer::run 协作式超时生效
    let c = Lang::C.table().unwrap();
    let mut long_comment = Vec::with_capacity(8 << 20);
    long_comment.extend_from_slice(b"/* ");
    long_comment.resize((8 << 20) - 3, b'a');
    long_comment.extend_from_slice(b"*/\nint x;\n");

    let t0 = std::time::Instant::now();
    let tree = parse(
        c,
        &long_comment,
        Limits {
            max_millis: 1,
            ..Limits::default()
        },
    );
    let el = t0.elapsed();
    if tree.truncated {
        assert_eq!(tree.arena.nodes.len(), 1);
        assert!(el.as_millis() < 250, "timeout took too long: {el:?}");
    }

    // 2. 外部扫描器（Bash heredoc / Rust raw string）长跨度下协作式超时
    let bash = Lang::Bash.table().unwrap();
    let mut heredoc = Vec::with_capacity(6 << 20);
    heredoc.extend_from_slice(b"cat <<'EOF'\n");
    heredoc.resize((6 << 20) - 5, b'x');
    heredoc.extend_from_slice(b"\nEOF\n");
    let t1 = std::time::Instant::now();
    let bt = parse(
        bash,
        &heredoc,
        Limits {
            max_millis: 1,
            ..Limits::default()
        },
    );
    let bel = t1.elapsed();
    if bt.truncated {
        assert_eq!(bt.arena.nodes.len(), 1);
        assert!(
            bel.as_millis() < 250,
            "scanner timeout took too long: {bel:?}"
        );
    }
}

#[test]
fn single_line_column_scanners_and_invalid_utf8_do_not_stall_or_panic() {
    // 单行 150 KiB Elm / OCaml / Bash 输入，确认 Lexer::column 不退化为 O(N^2)
    let elm = Lang::Elm.table().unwrap();
    let mut line = String::from("x = ");
    for i in 0..15_000 {
        if i > 0 {
            line.push_str(" + ");
        }
        line.push('a');
    }
    let t0 = std::time::Instant::now();
    let et = parse(elm, line.as_bytes(), Limits::default());
    assert!(!et.truncated);
    assert!(
        t0.elapsed().as_millis() < 1000,
        "single line took {:?}",
        t0.elapsed()
    );

    // 非法 UTF-8 混合序列与超深嵌套
    let bad_utf8: Vec<&[u8]> = vec![
        b"\xff\xfe\xfd",
        b"fn f() { let s = \"\xed\xa0\x80\xf4\x90\x80\x80\"; }",
        b"def g():\n    x = b'\xc0\xaf'\n",
        b"\x80\x81\xf0\x9f",
    ];
    for (tag, lang) in [
        ("rust", Lang::Rust),
        ("python", Lang::Python),
        ("c", Lang::C),
        ("bash", Lang::Bash),
    ] {
        let t = lang.table().unwrap();
        for b in &bad_utf8 {
            let tree = parse(t, b, Limits::default());
            assert_ne!(tree.root, NONE);
            let _ = tree.to_sexp(t);
            let _ = outline(tag, &String::from_utf8_lossy(b));
        }
    }
}

#[test]
fn triggers_stack_and_arena_compaction_cleanly() {
    // 生成足够多函数使 Arena 超过 MIN_TRIGGER (200_000) 从而触发 compact_stack 与 compact_arena
    let c = Lang::C.table().unwrap();
    let mut src = String::with_capacity(600_000);
    for i in 0..15_000 {
        src.push_str(&format!("int f{i}(int a) {{ return a + 1; }}\n"));
    }
    let tree = parse(c, src.as_bytes(), Limits::default());
    assert!(!tree.truncated);
    assert!(!tree.has_error());
    assert_eq!(tree.input_len, src.len());
    // 验证大树遍历与 outline 提取一致且无损拼接
    let out = outline("c", &src).expect("c outline");
    assert_eq!(
        out.sections
            .iter()
            .map(|s| &src[s.start..s.end])
            .collect::<String>(),
        src
    );
}
