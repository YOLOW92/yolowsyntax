use yolowsyntax::{Limits, api::parse, langs::Lang, outline};

fn main() {
    let rust_src = r#"
use std::collections::HashMap;

pub struct Config {
    pub limit: usize,
}

pub fn build_index(items: &[&str]) -> HashMap<String, usize> {
    items.iter().enumerate().map(|(i, s)| ((*s).to_string(), i)).collect()
}
"#;

    let out = outline("rust", rust_src).expect("rust language is supported");
    println!("imports: {:?}", out.imports);
    for sym in &out.symbols {
        println!("{:?}: {}", sym.kind, sym.name);
    }
    println!("sections: {}", out.sections.len());

    let lang = Lang::from_tag("rust").unwrap().table().unwrap();
    let tree = parse(lang, rust_src.as_bytes(), Limits::default());
    println!(
        "has_error: {}, truncated: {}",
        tree.has_error(),
        tree.truncated
    );
}
