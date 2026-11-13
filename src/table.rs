//! 语法表的内存格式与紧凑二进制序列化。
//!
//! 表由 `convert` 从 tree-sitter 生成的 `parser.c` 转换而来，运行时只读本模块的结构。

/// 解析动作表中的一个动作。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// 动作表入口头：后面紧跟 n 个动作。
    Header(u16),
    Shift {
        state: u16,
        extra: bool,
        repetition: bool,
    },
    Reduce {
        symbol: u16,
        count: u16,
        dprec: i16,
        prod: u16,
    },
    Accept,
    Recover,
}

/// 词法状态机里一段连续码点的行为（码点域 0..=0x110000，0x110000 表示输入结束）。
/// 紧凑 8 字节布局：`packed` 低 14 位存目标状态，高 2 位存动作类型（0 结束，1 前进，2 跳过）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seg {
    pub lo: u32,
    /// 走到这一段之前已经执行过的 ACCEPT_TOKEN 符号，`NO_SYM` 表示没有。
    pub accept: u16,
    pub packed: u16,
}

impl Seg {
    #[inline]
    pub const fn new(lo: u32, accept: u16, kind: u8, target: u16) -> Self {
        Self {
            lo,
            accept,
            packed: ((kind as u16) << 14) | (target & 0x3FFF),
        }
    }
    #[inline]
    pub const fn kind(self) -> u8 {
        (self.packed >> 14) as u8
    }
    #[inline]
    pub const fn target(self) -> u16 {
        self.packed & 0x3FFF
    }
}

pub const NO_SYM: u16 = 0xFFFF;
pub const EOF_CP: u32 = 0x110000;

#[derive(Clone, Debug, Default)]
pub(crate) struct LexAscii(Vec<u8>);

impl PartialEq for LexAscii {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for LexAscii {}

#[derive(Clone, Debug, Default)]
pub(crate) struct DenseTable {
    states: usize,
    stride: usize,
    data: Vec<u16>,
}

impl PartialEq for DenseTable {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for DenseTable {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LexTable {
    pub starts: Vec<u32>,
    pub segs: Vec<Seg>,
    pub(crate) ascii: LexAscii,
}

impl LexTable {
    /// 预先为 ASCII 码点（0..128）构造每状态 128 字节直查索引。
    pub(crate) fn build_accel(&mut self) {
        let sc = self.state_count();
        self.ascii.0.clear();
        if sc == 0 {
            return;
        }
        self.ascii.0.resize(sc * 128, 255);
        for s in 0..sc {
            let a = self.starts[s] as usize;
            let b = self.starts[s + 1] as usize;
            if a >= b {
                continue;
            }
            let row = &mut self.ascii.0[s * 128..(s + 1) * 128];
            let mut pos = 0usize;
            let mut last_k = 0usize;
            for k in 0..(b - a) {
                let lo = self.segs[a + k].lo as usize;
                let end = lo.min(128);
                if end > pos && k > 0 {
                    let v = if last_k < 255 { last_k as u8 } else { 255 };
                    row[pos..end].fill(v);
                    pos = end;
                } else if k == 0 && end > 0 {
                    pos = end;
                }
                if lo >= 128 {
                    break;
                }
                last_k = k;
            }
            if pos < 128 {
                let v = if last_k < 255 { last_k as u8 } else { 255 };
                row[pos..128].fill(v);
            }
        }
    }

    /// O(1) 查 ASCII 字节对应词法段（仅在 `byte < 128` 且 `state < state_count()` 时使用）。
    #[inline]
    pub(crate) fn find_ascii(&self, state: u16, byte: u8) -> Seg {
        let st = state as usize;
        let idx = self
            .ascii
            .0
            .get(st * 128 + byte as usize)
            .copied()
            .unwrap_or(255);
        if idx < 255 {
            self.segs[self.starts[st] as usize + idx as usize]
        } else {
            self.find(state, byte as u32)
        }
    }

    /// 在某个词法状态里按码点查段。
    pub fn find(&self, state: u16, cp: u32) -> Seg {
        let (a, b) = (
            self.starts[state as usize] as usize,
            self.starts[state as usize + 1] as usize,
        );
        let s = &self.segs[a..b];
        if s.is_empty() {
            return Seg::new(0, NO_SYM, 0, 0);
        }
        let i = s.partition_point(|x| x.lo <= cp);
        s[i.saturating_sub(1)]
    }
    pub fn state_count(&self) -> usize {
        self.starts.len().saturating_sub(1)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Language {
    pub name: String,
    pub abi_version: u32,
    pub symbol_count: u32,
    pub alias_count: u32,
    pub token_count: u32,
    pub external_token_count: u32,
    pub state_count: u32,
    pub large_state_count: u32,
    pub production_id_count: u32,
    pub field_count: u32,
    pub max_alias_sequence_length: u32,
    pub keyword_capture_token: u16,
    pub symbol_names: Vec<String>,
    pub public_symbol_map: Vec<u16>,
    /// 位 0 可见，位 1 具名，位 2 supertype。
    pub symbol_metadata: Vec<u8>,
    pub field_names: Vec<String>,
    /// 每个 production 的字段映射切片 (index, length)。
    pub field_map_slices: Vec<(u16, u16)>,
    /// (字段 id, 子节点序号, 是否继承)
    pub field_map_entries: Vec<(u16, u8, bool)>,
    pub alias_sequences: Vec<u16>,
    pub non_terminal_alias_map: Vec<u16>,
    pub primary_state_ids: Vec<u16>,
    /// 每个解析状态的词法状态。
    pub lex_modes: Vec<u16>,
    /// 每个解析状态使用的保留字集合编号（0 表示没有）。
    pub reserved_set: Vec<u8>,
    /// 保留字集合，按 `max_reserved` 个一行展平，行内遇到 0 结束。
    pub reserved_words: Vec<u16>,
    pub max_reserved: u32,
    pub state_start: Vec<u32>,
    /// 高 16 位符号，低 16 位值，同一状态内按符号升序。
    pub entries: Vec<u32>,
    pub actions: Vec<Action>,
    pub lex: LexTable,
    pub keyword_lex: LexTable,
    /// 每个解析状态的外部词法状态（0 表示没有外部记号可用）。
    pub lex_ext: Vec<u16>,
    /// 外部记号序号到符号的映射。
    pub ext_symbol_map: Vec<u16>,
    /// 外部词法状态的有效记号表：按  一行展平，第 0 行全假（行宽为外部记号数）。
    pub ext_valid: Vec<bool>,
    pub(crate) dense: DenseTable,
}

pub const SYM_END: u16 = 0;
pub const SYM_ERROR: u16 = 0xFFFF;
pub const SYM_ERROR_REPEAT: u16 = 0xFFFE;

impl Language {
    /// 构建热状态 O(1) 扁平查找表与词法 ASCII 索引（内存上限 1 MiB）。
    pub(crate) fn build_accel(&mut self) {
        self.lex.build_accel();
        self.keyword_lex.build_accel();
        let stride = (self.symbol_count + self.alias_count) as usize;
        let cap_states = (512 * 1024) / stride.max(1);
        let dense_states = (self.large_state_count as usize)
            .min(self.state_count as usize)
            .min(cap_states);
        if dense_states == 0 || stride == 0 {
            self.dense = DenseTable::default();
            return;
        }
        let mut data = vec![0u16; dense_states * stride];
        for s in 0..dense_states {
            let (a, b) = (
                self.state_start[s] as usize,
                self.state_start[s + 1] as usize,
            );
            let row = &mut data[s * stride..(s + 1) * stride];
            for &x in &self.entries[a..b] {
                let sym = (x >> 16) as usize;
                if sym < stride {
                    row[sym] = (x & 0xFFFF) as u16;
                }
            }
        }
        self.dense = DenseTable {
            states: dense_states,
            stride,
            data,
        };
    }

    /// 对应官方 `ts_language_lookup`：终结符得到动作下标，非终结符得到后继状态。
    #[inline]
    pub fn lookup(&self, state: u16, symbol: u16) -> u16 {
        let st = state as usize;
        if st < self.dense.states {
            let sym = symbol as usize;
            return if sym < self.dense.stride {
                self.dense.data[st * self.dense.stride + sym]
            } else {
                0
            };
        }
        let (a, b) = (
            self.state_start[st] as usize,
            self.state_start[st + 1] as usize,
        );
        let e = &self.entries[a..b];
        if e.len() <= 16 {
            let sym = symbol as u32;
            for &x in e {
                let s = x >> 16;
                if s >= sym {
                    return if s == sym { (x & 0xFFFF) as u16 } else { 0 };
                }
            }
            return 0;
        }
        let i = e.partition_point(|x| (x >> 16) < symbol as u32);
        if i < e.len() && (e[i] >> 16) == symbol as u32 {
            (e[i] & 0xFFFF) as u16
        } else {
            0
        }
    }
    #[inline]
    pub fn has_actions(&self, state: u16, symbol: u16) -> bool {
        self.lookup(state, symbol) != 0
    }
    #[inline]
    pub(crate) fn actions_at(&self, idx: u16) -> &[Action] {
        let idx = idx as usize;
        if idx == 0 {
            return &[];
        }
        match self.actions.get(idx) {
            Some(Action::Header(n)) => &self.actions[idx + 1..idx + 1 + *n as usize],
            _ => &[],
        }
    }
    /// 遍历某解析状态下所有非 EOF 终结符的动作切片（兼容稠密直查表与稀疏 `entries`）。
    pub(crate) fn for_each_token_actions(&self, state: u16, mut f: impl FnMut(&[Action])) {
        let st = state as usize;
        if st < self.dense.states {
            let row = &self.dense.data[st * self.dense.stride..];
            let tc = (self.token_count as usize).min(self.dense.stride);
            for &idx in &row[1..tc] {
                if idx != 0 {
                    f(self.actions_at(idx));
                }
            }
            return;
        }
        let (s, e) = (
            self.state_start[st] as usize,
            self.state_start[st + 1] as usize,
        );
        for &ent in &self.entries[s..e] {
            let sym = (ent >> 16) as u16;
            if sym == 0 || sym as u32 >= self.token_count {
                continue;
            }
            f(self.actions_at((ent & 0xFFFF) as u16));
        }
    }
    /// 官方 `ts_language_table_entry`：错误符号没有动作。
    #[inline]
    pub fn actions_for(&self, state: u16, symbol: u16) -> &[Action] {
        if symbol == SYM_ERROR || symbol == SYM_ERROR_REPEAT {
            return &[];
        }
        self.actions_at(self.lookup(state, symbol))
    }
    pub fn has_reduce_action(&self, state: u16, symbol: u16) -> bool {
        matches!(
            self.actions_for(state, symbol).first(),
            Some(Action::Reduce { .. })
        )
    }
    pub fn next_state(&self, state: u16, symbol: u16) -> u16 {
        if symbol == SYM_ERROR || symbol == SYM_ERROR_REPEAT {
            return 0;
        }
        if (symbol as u32) < self.token_count {
            if let Some(Action::Shift {
                state: s, extra, ..
            }) = self.actions_for(state, symbol).last()
            {
                return if *extra { state } else { *s };
            }
            0
        } else {
            self.lookup(state, symbol)
        }
    }
    /// 官方 `ts_language_is_reserved_word`。
    pub fn is_reserved(&self, state: u16, sym: u16) -> bool {
        let id = self.reserved_set.get(state as usize).copied().unwrap_or(0) as usize;
        if id == 0 || self.max_reserved == 0 {
            return false;
        }
        let m = self.max_reserved as usize;
        let row = self.reserved_words.get(id * m..(id + 1) * m).unwrap_or(&[]);
        for &w in row {
            if w == sym {
                return true;
            }
            if w == 0 {
                break;
            }
        }
        false
    }
    /// 解析状态对应的外部词法状态（没有外部记号返回 0）。
    #[inline]
    pub fn ext_state(&self, parse_state: u16) -> u16 {
        self.lex_ext.get(parse_state as usize).copied().unwrap_or(0)
    }
    /// 某个外部词法状态下各外部记号是否有效。
    pub fn ext_valid_row(&self, ext_state: u16) -> &[bool] {
        let ec = self.external_token_count as usize;
        self.ext_valid
            .get(ext_state as usize * ec..(ext_state as usize + 1) * ec)
            .unwrap_or(&[])
    }
    pub fn visible(&self, sym: u16) -> bool {
        self.meta(sym) & 1 != 0
    }
    pub fn named(&self, sym: u16) -> bool {
        self.meta(sym) & 2 != 0
    }
    fn meta(&self, sym: u16) -> u8 {
        match sym {
            SYM_ERROR => 3,
            SYM_ERROR_REPEAT => 0,
            _ => self.symbol_metadata.get(sym as usize).copied().unwrap_or(0),
        }
    }
    pub fn symbol_name(&self, sym: u16) -> &str {
        match sym {
            SYM_ERROR => "ERROR",
            SYM_ERROR_REPEAT => "_error_repeat",
            _ => self
                .symbol_names
                .get(sym as usize)
                .map(|s| s.as_str())
                .unwrap_or("?"),
        }
    }
    pub fn public_symbol(&self, sym: u16) -> u16 {
        self.public_symbol_map
            .get(sym as usize)
            .copied()
            .unwrap_or(sym)
    }
    pub fn field_name(&self, id: u16) -> &str {
        self.field_names
            .get(id as usize)
            .map(|s| s.as_str())
            .unwrap_or("")
    }
    pub fn alias_sequence(&self, prod: u16) -> Option<&[u16]> {
        if prod == 0 {
            return None;
        }
        let m = self.max_alias_sequence_length as usize;
        let a = prod as usize * m;
        self.alias_sequences.get(a..a + m)
    }

    // ---- 二进制序列化 ----
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = W(Vec::new());
        w.0.extend_from_slice(b"PSLG2\0\0\0");
        w.str(&self.name);
        for v in [
            self.abi_version,
            self.symbol_count,
            self.alias_count,
            self.token_count,
            self.external_token_count,
            self.state_count,
            self.large_state_count,
            self.production_id_count,
            self.field_count,
            self.max_alias_sequence_length,
            self.keyword_capture_token as u32,
        ] {
            w.u32(v);
        }
        w.strs(&self.symbol_names);
        w.u16s(&self.public_symbol_map);
        w.bytes(&self.symbol_metadata);
        w.strs(&self.field_names);
        w.u32(self.field_map_slices.len() as u32);
        for (a, b) in &self.field_map_slices {
            w.u16(*a);
            w.u16(*b);
        }
        w.u32(self.field_map_entries.len() as u32);
        for (f, i, inh) in &self.field_map_entries {
            w.u16(*f);
            w.0.push(*i);
            w.0.push(*inh as u8);
        }
        w.u16s(&self.alias_sequences);
        w.u16s(&self.non_terminal_alias_map);
        w.u16s(&self.primary_state_ids);
        w.u16s(&self.lex_modes);
        w.bytes(&self.reserved_set);
        w.u16s(&self.reserved_words);
        w.u32(self.max_reserved);
        w.u32s(&self.state_start);
        w.u32s(&self.entries);
        w.u32(self.actions.len() as u32);
        for a in &self.actions {
            let (t, f, p): (u8, u8, [u16; 4]) = match *a {
                Action::Header(n) => (0, 0, [n, 0, 0, 0]),
                Action::Shift {
                    state,
                    extra,
                    repetition,
                } => (1, extra as u8 | (repetition as u8) << 1, [state, 0, 0, 0]),
                Action::Reduce {
                    symbol,
                    count,
                    dprec,
                    prod,
                } => (2, 0, [symbol, count, dprec as u16, prod]),
                Action::Accept => (3, 0, [0; 4]),
                Action::Recover => (4, 0, [0; 4]),
            };
            w.0.push(t);
            w.0.push(f);
            for x in p {
                w.u16(x);
            }
        }
        for t in [&self.lex, &self.keyword_lex] {
            w.u32s(&t.starts);
            w.u32(t.segs.len() as u32);
            for s in &t.segs {
                w.u32(s.lo);
                w.u16(s.accept);
                w.u16(s.target());
                w.0.push(s.kind());
            }
        }
        // 外部扫描器相关的尾部分段（旧表没有）
        w.u16s(&self.lex_ext);
        w.u16s(&self.ext_symbol_map);
        w.bytes(&self.ext_valid.iter().map(|b| *b as u8).collect::<Vec<_>>());
        w.0
    }

    /// 运行时专用加载：构造好稠密直查表后裁掉被覆盖的 `entries` 前缀以压缩常驻内存。
    pub(crate) fn from_bytes_runtime(b: &[u8]) -> Result<Language, String> {
        let mut l = Self::from_bytes(b)?;
        let ds = l.dense.states;
        if ds > 0 && ds <= l.state_count as usize {
            let drop_idx = l.state_start[ds] as usize;
            if drop_idx > 0 && drop_idx <= l.entries.len() {
                l.entries = l.entries[drop_idx..].to_vec();
                let d = drop_idx as u32;
                for x in &mut l.state_start[ds..] {
                    *x -= d;
                }
                l.state_start[..ds].fill(0);
            }
        }
        Ok(l)
    }

    pub fn from_bytes(b: &[u8]) -> Result<Language, String> {
        let mut r = R { b, p: 0 };
        if b.len() < 8 || &b[..8] != b"PSLG2\0\0\0" {
            return Err("语法表魔数不符".into());
        }
        r.p = 8;
        let mut l = Language {
            name: r.str()?,
            ..Default::default()
        };
        l.abi_version = r.u32()?;
        l.symbol_count = r.u32()?;
        l.alias_count = r.u32()?;
        l.token_count = r.u32()?;
        l.external_token_count = r.u32()?;
        l.state_count = r.u32()?;
        l.large_state_count = r.u32()?;
        l.production_id_count = r.u32()?;
        l.field_count = r.u32()?;
        l.max_alias_sequence_length = r.u32()?;
        l.keyword_capture_token = r.u32()? as u16;
        l.symbol_names = r.strs()?;
        l.public_symbol_map = r.u16s()?;
        l.symbol_metadata = r.bytes()?;
        l.field_names = r.strs()?;
        let n = r.u32()? as usize;
        let bytes = r.take(n.checked_mul(4).ok_or("长度溢出")?)?;
        l.field_map_slices.reserve(n);
        for c in bytes.as_chunks::<4>().0 {
            l.field_map_slices.push((
                u16::from_le_bytes([c[0], c[1]]),
                u16::from_le_bytes([c[2], c[3]]),
            ));
        }
        let n = r.u32()? as usize;
        let bytes = r.take(n.checked_mul(4).ok_or("长度溢出")?)?;
        l.field_map_entries.reserve(n);
        for c in bytes.as_chunks::<4>().0 {
            l.field_map_entries
                .push((u16::from_le_bytes([c[0], c[1]]), c[2], c[3] != 0));
        }
        l.alias_sequences = r.u16s()?;
        l.non_terminal_alias_map = r.u16s()?;
        l.primary_state_ids = r.u16s()?;
        l.lex_modes = r.u16s()?;
        l.reserved_set = r.bytes()?;
        l.reserved_words = r.u16s()?;
        l.max_reserved = r.u32()?;
        l.state_start = r.u32s()?;
        l.entries = r.u32s()?;
        let n = r.u32()? as usize;
        let bytes = r.take(n.checked_mul(10).ok_or("长度溢出")?)?;
        l.actions.reserve(n);
        for c in bytes.as_chunks::<10>().0 {
            let t = c[0];
            let f = c[1];
            let p0 = u16::from_le_bytes([c[2], c[3]]);
            let p1 = u16::from_le_bytes([c[4], c[5]]);
            let p2 = u16::from_le_bytes([c[6], c[7]]);
            let p3 = u16::from_le_bytes([c[8], c[9]]);
            l.actions.push(match t {
                0 => Action::Header(p0),
                1 => Action::Shift {
                    state: p0,
                    extra: f & 1 != 0,
                    repetition: f & 2 != 0,
                },
                2 => Action::Reduce {
                    symbol: p0,
                    count: p1,
                    dprec: p2 as i16,
                    prod: p3,
                },
                3 => Action::Accept,
                4 => Action::Recover,
                _ => return Err("未知动作类型".into()),
            });
        }
        for k in 0..2 {
            let starts = r.u32s()?;
            let n = r.u32()? as usize;
            let bytes = r.take(n.checked_mul(9).ok_or("长度溢出")?)?;
            let mut segs = Vec::with_capacity(n);
            for c in bytes.as_chunks::<9>().0 {
                let lo = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                let accept = u16::from_le_bytes([c[4], c[5]]);
                let target = u16::from_le_bytes([c[6], c[7]]);
                let kind = c[8];
                if target > 0x3FFF || kind > 2 {
                    return Err("词法段越界".into());
                }
                segs.push(Seg::new(lo, accept, kind, target));
            }
            let t = LexTable {
                starts,
                segs,
                ascii: LexAscii::default(),
            };
            if k == 0 { l.lex = t } else { l.keyword_lex = t }
        }
        if r.p < b.len() {
            l.lex_ext = r.u16s()?;
            l.ext_symbol_map = r.u16s()?;
            l.ext_valid = r.bytes()?.into_iter().map(|x| x != 0).collect();
        }
        l.validate()?;
        l.build_accel();
        Ok(l)
    }

    /// 加载后的一致性检查，保证运行时的下标访问不会越界。
    fn validate(&self) -> Result<(), String> {
        let sc = self.state_count as usize;
        if self.state_start.len() != sc + 1
            || self.lex_modes.len() != sc
            || self.reserved_set.len() != sc
        {
            return Err("状态表长度不符".into());
        }
        if self.state_start.windows(2).any(|w| w[0] > w[1])
            || *self.state_start.last().unwrap() as usize > self.entries.len()
        {
            return Err("状态偏移不单调".into());
        }
        let total = (self.symbol_count + self.alias_count) as usize;
        if self.symbol_names.len() < total
            || self.symbol_metadata.len() < total
            || self.public_symbol_map.len() < total
        {
            return Err("符号表长度不符".into());
        }
        let ec = self.external_token_count as usize;
        if ec > 0 {
            if self.lex_ext.len() != sc
                || self.ext_symbol_map.len() != ec
                || !self.ext_valid.len().is_multiple_of(ec)
            {
                return Err("外部扫描器表长度不符".into());
            }
            let rows = self.ext_valid.len() / ec;
            if self.lex_ext.iter().any(|x| *x as usize >= rows.max(1)) {
                return Err("外部词法状态越界".into());
            }
        }
        let mut i = 0;
        while i < self.actions.len() {
            if let Action::Header(n) = self.actions[i] {
                i += 1 + n as usize;
            } else {
                i += 1;
            }
        }
        if i != self.actions.len() {
            return Err("动作表越界".into());
        }
        for t in [&self.lex, &self.keyword_lex] {
            if (t.starts.is_empty() || *t.starts.last().unwrap() as usize != t.segs.len())
                && !(t.starts.is_empty() && t.segs.is_empty())
            {
                return Err("词法表偏移不符".into());
            }
        }
        Ok(())
    }
}

struct W(Vec<u8>);
impl W {
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn bytes(&mut self, v: &[u8]) {
        self.u32(v.len() as u32);
        self.0.extend_from_slice(v);
    }
    fn str(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
    fn strs(&mut self, v: &[String]) {
        self.u32(v.len() as u32);
        for s in v {
            self.str(s);
        }
    }
    fn u16s(&mut self, v: &[u16]) {
        self.u32(v.len() as u32);
        for x in v {
            self.u16(*x);
        }
    }
    fn u32s(&mut self, v: &[u32]) {
        self.u32(v.len() as u32);
        for x in v {
            self.u32(*x);
        }
    }
}

struct R<'a> {
    b: &'a [u8],
    p: usize,
}
impl R<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let e = self
            .p
            .checked_add(n)
            .filter(|e| *e <= self.b.len())
            .ok_or("语法表被截断")?;
        let s = &self.b[self.p..e];
        self.p = e;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, String> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn bytes(&mut self) -> Result<Vec<u8>, String> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.to_vec())
    }
    fn str(&mut self) -> Result<String, String> {
        String::from_utf8(self.bytes()?).map_err(|e| e.to_string())
    }
    fn strs(&mut self) -> Result<Vec<String>, String> {
        let n = self.u32()? as usize;
        (0..n).map(|_| self.str()).collect()
    }
    fn u16s(&mut self) -> Result<Vec<u16>, String> {
        let n = self.u32()? as usize;
        let s = self.take(n.checked_mul(2).ok_or("长度溢出")?)?;
        Ok(s.as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect())
    }
    fn u32s(&mut self) -> Result<Vec<u32>, String> {
        let n = self.u32()? as usize;
        let s = self.take(n.checked_mul(4).ok_or("长度溢出")?)?;
        Ok(s.as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect())
    }
}
