//! 运行时词法器：在 UTF-8 输入上执行转换后的词法状态机。

use crate::table::{EOF_CP, LexTable, NO_SYM};
use crate::tree::Len;
use std::time::Instant;

pub struct Lexer<'a> {
    pub input: &'a [u8],
    pub pos: Len,
    pub token_start: Len,
    pub token_end: Option<Len>,
    pub result: u16,
    deadline: Option<Instant>,
    pub timed_out: bool,
    col_row: u32,
    col_byte: u32,
    col_val: u32,
}

/// 解码一个码点，非法字节当作 U+FFFD 前进一字节。
#[inline]
fn decode(b: &[u8], i: usize) -> (u32, usize) {
    let c = b[i];
    if c < 0x80 {
        return (c as u32, 1);
    }
    let (n, init) = match c {
        0xC2..=0xDF => (2, (c & 0x1F) as u32),
        0xE0..=0xEF => (3, (c & 0x0F) as u32),
        0xF0..=0xF4 => (4, (c & 0x07) as u32),
        _ => return (0xFFFD, 1),
    };
    if i + n > b.len() {
        return (0xFFFD, 1);
    }
    let mut v = init;
    for k in 1..n {
        let x = b[i + k];
        if x & 0xC0 != 0x80 {
            return (0xFFFD, 1);
        }
        v = v << 6 | (x & 0x3F) as u32;
    }
    let ok = match n {
        2 => v >= 0x80,
        3 => v >= 0x800 && !(0xD800..0xE000).contains(&v),
        _ => (0x10000..=0x10FFFF).contains(&v),
    };
    if ok { (v, n) } else { (0xFFFD, 1) }
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Lexer {
            input,
            pos: Len::default(),
            token_start: Len::default(),
            token_end: None,
            result: 0,
            deadline: None,
            timed_out: false,
            col_row: 0,
            col_byte: 0,
            col_val: 0,
        }
    }
    #[inline]
    pub fn set_deadline(&mut self, deadline: Option<Instant>) {
        self.deadline = deadline;
    }
    #[inline]
    pub fn check_deadline(&mut self) -> bool {
        if let Some(dl) = self.deadline
            && Instant::now() >= dl
        {
            self.timed_out = true;
            return true;
        }
        false
    }
    pub fn reset(&mut self, p: Len) {
        self.pos = p;
    }
    pub fn start(&mut self) {
        self.token_start = self.pos;
        self.token_end = None;
        self.result = 0;
    }
    pub fn eof(&self) -> bool {
        self.pos.bytes as usize >= self.input.len()
    }
    /// 当前前瞻码点，输入结束返回 EOF_CP。
    #[inline]
    pub fn look(&self) -> u32 {
        let i = self.pos.bytes as usize;
        if i >= self.input.len() {
            EOF_CP
        } else {
            decode(self.input, i).0
        }
    }
    pub fn advance(&mut self, skip: bool) {
        let i = self.pos.bytes as usize;
        if i < self.input.len() {
            let (c, n) = decode(self.input, i);
            self.pos.bytes += n as u32;
            if c == '\n' as u32 {
                self.pos.rows += 1;
            }
        }
        if skip {
            self.token_start = self.pos;
        }
    }
    /// 当前位置在行内的列号（按码点计，对应官方 `get_column`）。
    pub fn column(&mut self) -> u32 {
        let end = (self.pos.bytes as usize).min(self.input.len());
        let (mut i, mut n) = if self.pos.rows == self.col_row && end >= self.col_byte as usize {
            (self.col_byte as usize, self.col_val)
        } else {
            let start = self.input[..end]
                .iter()
                .rposition(|b| *b == b'\n')
                .map_or(0, |k| k + 1);
            (start, 0u32)
        };
        if self.input[i..end].is_ascii() {
            n += (end - i) as u32;
        } else {
            while i < end {
                i += decode(self.input, i).1;
                n += 1;
            }
        }
        self.col_row = self.pos.rows;
        self.col_byte = end as u32;
        self.col_val = n;
        n
    }
    pub fn mark_end(&mut self) {
        self.token_end = Some(self.pos);
    }
    pub fn finish(&mut self) {
        if self.token_end.is_none() {
            self.mark_end();
        }
    }

    /// 执行一个词法函数，成功返回 true，结果符号在 `self.result`。
    pub fn run(&mut self, t: &LexTable, mut state: u16) -> bool {
        let sc = t.state_count();
        if t.starts.is_empty() || state as usize >= sc {
            return false;
        }
        let input = self.input;
        let len = input.len();
        let mut found = false;
        // 在输入末尾“前进”不会移动位置；词法表若在末尾处成环就永远不会停，所以限制连续次数
        let mut stuck = 0u32;
        let mut ticks = 0u32;
        loop {
            ticks = ticks.wrapping_add(1);
            if ticks.is_multiple_of(4096) && self.check_deadline() {
                return false;
            }
            let i = self.pos.bytes as usize;
            if i < len {
                let b = input[i];
                if b < 0x80 {
                    let seg = t.find_ascii(state, b);
                    if seg.accept != NO_SYM {
                        found = true;
                        self.result = seg.accept;
                        self.token_end = Some(self.pos);
                    }
                    match seg.kind() {
                        1 => {
                            self.pos.bytes = (i + 1) as u32;
                            if b == b'\n' {
                                self.pos.rows += 1;
                            }
                        }
                        2 => {
                            self.pos.bytes = (i + 1) as u32;
                            if b == b'\n' {
                                self.pos.rows += 1;
                            }
                            self.token_start = self.pos;
                        }
                        _ => return found,
                    }
                    stuck = 0;
                    let tgt = seg.target();
                    if tgt as usize >= sc {
                        return found;
                    }
                    state = tgt;
                    continue;
                }
            }
            let seg = t.find(state, self.look());
            if seg.accept != NO_SYM {
                found = true;
                self.result = seg.accept;
                self.mark_end();
            }
            let before = self.pos.bytes;
            match seg.kind() {
                1 => self.advance(false),
                2 => self.advance(true),
                _ => return found,
            }
            if self.pos.bytes == before {
                stuck += 1;
                if stuck > 8 {
                    return found;
                }
            } else {
                stuck = 0;
            }
            let tgt = seg.target();
            if tgt as usize >= sc {
                return found;
            }
            state = tgt;
        }
    }
}
