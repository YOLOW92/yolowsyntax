//! 与 tree-sitter 语法兼容的纯 Rust 解析运行时（不含增量解析）。
//!
//! 语法表由 `ts-convert` 从各语法的 `parser.c` 转换，内嵌在 `tables/`；来源和许可见 `THIRD-PARTY.md`。
#![forbid(unsafe_code)]

pub mod api;
#[cfg(feature = "tools")]
pub mod cdata;
pub mod compact;
#[cfg(feature = "tools")]
pub mod convert;
#[cfg(feature = "tools")]
pub mod corpus;
pub mod extract;
pub mod langs;
#[cfg(feature = "tools")]
pub mod lexconv;
pub mod lexer;
pub mod parser;
pub mod recover;
pub mod scanner;
pub mod stack;
pub mod table;
pub mod tree;

pub use extract::{Outline, Section, Symbol, SymbolKind, outline};

/// 解析的协作式资源上限：栈深、GLR 版本数、节点数、步数与可选墙钟时间预算。
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_stack_depth: usize,
    /// 正常最大活跃 GLR 版本数（归约过程允许少量内部溢出缓冲，随后裁剪回该上限；0 返回退化树）。
    pub max_versions: usize,
    /// 最大语法树节点数（0 返回退化树）。
    pub max_nodes: usize,
    /// 解析步数上限；0 表示按输入规模自动算（每字节 400 步加 20 万步）。超限返回退化树。
    pub max_steps: u64,
    /// 协作式墙钟时间上限（毫秒）；0 表示不限。超限返回退化树。
    pub max_millis: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_stack_depth: 4096,
            max_versions: 6,
            max_nodes: 4_000_000,
            max_steps: 0,
            max_millis: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn limits_default_is_bounded() {
        assert!(super::Limits::default().max_versions > 0);
    }
}
