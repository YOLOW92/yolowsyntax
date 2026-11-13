//! 用法：ts-convert <parser.c> <输出.pslg> [语言名]
fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 3 {
        eprintln!("用法: ts-convert <parser.c> <输出> [语言名]");
        std::process::exit(2);
    }
    let text = std::fs::read_to_string(&a[1]).expect("读取 parser.c 失败");
    let name = a.get(3).cloned().unwrap_or_else(|| "unknown".into());
    match yolowsyntax::convert::convert(&text, &name) {
        Ok(l) => {
            let raw = l.to_bytes();
            let out = if a[2].ends_with(".z") {
                yolowzstd::compress_with(&raw, &yolowzstd::CompressOptions::new(19))
            } else {
                raw.clone()
            };
            std::fs::write(&a[2], &out).expect("写出失败");
            println!(
                "{}: 状态 {} 符号 {} 动作 {} 词法段 {}+{} -> {} 字节",
                l.name,
                l.state_count,
                l.symbol_count,
                l.actions.len(),
                l.lex.segs.len(),
                l.keyword_lex.segs.len(),
                out.len()
            );
        }
        Err(e) => {
            eprintln!("转换失败: {e}");
            std::process::exit(1);
        }
    }
}
