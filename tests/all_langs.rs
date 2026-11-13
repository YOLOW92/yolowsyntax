//! 全 41 种内嵌语言表的加载、容错与资源上限测试。

use yolowsyntax::Limits;
use yolowsyntax::api::parse;
use yolowsyntax::langs::Lang;
use yolowsyntax::outline;

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

#[test]
fn all_41_tables_load_and_parse_empty_and_garbage() {
    assert_eq!(ALL_TAGS.len(), 41);
    for &tag in ALL_TAGS {
        let lang = Lang::from_tag(tag).unwrap_or_else(|| panic!("missing tag {tag}"));
        let table = lang.table().unwrap_or_else(|| panic!("failed table {tag}"));
        let empty = parse(table, b"", Limits::default());
        assert!(!empty.truncated, "{tag} empty truncated");
        let _ = empty.to_sexp(table);

        let garbage = parse(table, b"\xff\xfe{{{[[\n\0\r\n}}]]", Limits::default());
        assert!(!garbage.to_sexp(table).is_empty(), "{tag} garbage sexp");
    }
}

#[test]
fn limits_enforce_stack_node_and_step_bounds() {
    let rust = Lang::from_tag("rust").unwrap().table().unwrap();
    let deep = format!("fn f() {{ {} }}", "(".repeat(4000));
    let t_stack = parse(
        rust,
        deep.as_bytes(),
        Limits {
            max_stack_depth: 64,
            ..Limits::default()
        },
    );
    assert!(t_stack.truncated);

    let many_stmts = "let x = 1;\n".repeat(2000);
    let t_nodes = parse(
        rust,
        many_stmts.as_bytes(),
        Limits {
            max_nodes: 200,
            ..Limits::default()
        },
    );
    assert!(t_nodes.truncated);

    let t_steps = parse(
        rust,
        many_stmts.as_bytes(),
        Limits {
            max_steps: 50,
            ..Limits::default()
        },
    );
    assert!(t_steps.truncated);
}

#[test]
fn outline_non_symbol_languages_return_none_without_panic() {
    for tag in [
        "css",
        "html",
        "nix",
        "dockerfile",
        "hcl",
        "vue",
        "svelte",
        "make",
        "scheme",
    ] {
        assert!(outline(tag, "/* empty */").is_none(), "{tag}");
    }
}
