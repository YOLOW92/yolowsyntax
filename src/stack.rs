//! GLR 图结构栈（对应官方 `stack.c`）：多个版本共享栈节点，相同状态的版本可合并。

use crate::table::SYM_ERROR_REPEAT;
use crate::tree::{Arena, Len, NONE};

const MAX_LINK_COUNT: usize = 8;
const MAX_ITERATOR_COUNT: usize = 64;
pub const ERROR_STATE: u16 = 0;

#[derive(Clone, Copy)]
pub struct Link {
    pub node: u32,
    pub tree: u32,
    pub pending: bool,
}

#[derive(Clone, Copy)]
pub struct SNode {
    pub state: u16,
    pub link_len: u8,
    pub pos: Len,
    pub first: Link,
    /// `link_len >= 2` 时指向 `Stack::extra_links` 下标，否则为 NONE。
    pub extra_links: u32,
    pub error_cost: u32,
    pub node_count: u32,
    pub dprec: i32,
    pub depth: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Active,
    Paused,
    Halted,
}

#[derive(Clone, Copy)]
pub struct SumEntry {
    pub pos: Len,
    pub depth: u32,
    pub state: u16,
}

#[derive(Clone)]
pub struct Head {
    pub node: u32,
    pub status: Status,
    pub node_count_at_last_error: u32,
    pub summary: Option<Vec<SumEntry>>,
    pub paused_lookahead: u32,
    /// 最后一个外部记号之后的扫描器状态编号（0 为初始空状态）。
    pub ext: u32,
}

pub struct Slice {
    pub version: usize,
    pub trees: Vec<u32>,
}

struct Iter {
    node: u32,
    trees: Vec<u32>,
    count: u32,
    pending: bool,
}

pub struct Stack {
    pub nodes: Vec<SNode>,
    pub extra_links: Vec<[Link; MAX_LINK_COUNT - 1]>,
    pub heads: Vec<Head>,
    base: u32,
    pub(crate) scratch_nodes: Vec<SNode>,
    pub(crate) scratch_extra: Vec<[Link; MAX_LINK_COUNT - 1]>,
    pub(crate) scratch_smap: Vec<u32>,
}

fn subtree_node_count(a: &Arena, t: u32) -> u32 {
    let n = a.n(t);
    let mut c = n.visible_descendants;
    if n.visible() {
        c += 1;
    }
    if n.sym == SYM_ERROR_REPEAT {
        c += 1;
    }
    c
}

fn equivalent(a: &Arena, l: u32, r: u32) -> bool {
    if l == r {
        return true;
    }
    if l == NONE || r == NONE {
        return false;
    }
    let (ln, rn) = (a.n(l), a.n(r));
    if ln.sym != rn.sym {
        return false;
    }
    if ln.cost() > 0 && rn.cost() > 0 {
        return true;
    }
    ln.padding.bytes == rn.padding.bytes
        && ln.size.bytes == rn.size.bytes
        && ln.child_count == rn.child_count
        && ln.extra() == rn.extra()
}

const EMPTY_LINK: Link = Link {
    node: 0,
    tree: NONE,
    pending: false,
};

impl Stack {
    pub fn new() -> Stack {
        let mut s = Stack {
            nodes: Vec::new(),
            extra_links: Vec::new(),
            heads: Vec::new(),
            base: 0,
            scratch_nodes: Vec::new(),
            scratch_extra: Vec::new(),
            scratch_smap: Vec::new(),
        };
        s.nodes.push(SNode {
            state: 1,
            link_len: 0,
            pos: Len::default(),
            first: EMPTY_LINK,
            extra_links: NONE,
            error_cost: 0,
            node_count: 0,
            dprec: 0,
            depth: 0,
        });
        s.clear();
        s
    }

    #[inline]
    pub fn link_count(&self, node: u32) -> usize {
        self.nodes[node as usize].link_len as usize
    }

    #[inline]
    pub fn link_at(&self, node: u32, idx: usize) -> Link {
        let n = &self.nodes[node as usize];
        if idx == 0 {
            n.first
        } else {
            self.extra_links[n.extra_links as usize][idx - 1]
        }
    }

    #[inline]
    fn set_link(&mut self, node: u32, idx: usize, link: Link) {
        let n = &mut self.nodes[node as usize];
        if idx == 0 {
            n.first = link;
        } else {
            self.extra_links[n.extra_links as usize][idx - 1] = link;
        }
    }

    #[inline]
    fn push_node_link(&mut self, node: u32, link: Link) {
        let len = self.nodes[node as usize].link_len as usize;
        if len == 0 {
            let n = &mut self.nodes[node as usize];
            n.first = link;
            n.link_len = 1;
        } else if len < MAX_LINK_COUNT {
            if len == 1 {
                let slot = self.extra_links.len() as u32;
                self.extra_links.push([EMPTY_LINK; MAX_LINK_COUNT - 1]);
                self.extra_links[slot as usize][0] = link;
                let n = &mut self.nodes[node as usize];
                n.extra_links = slot;
                n.link_len = 2;
            } else {
                let slot = self.nodes[node as usize].extra_links as usize;
                self.extra_links[slot][len - 1] = link;
                self.nodes[node as usize].link_len = (len + 1) as u8;
            }
        }
    }

    #[inline]
    fn copy_links(&self, node: u32) -> ([Link; MAX_LINK_COUNT], usize) {
        let n = &self.nodes[node as usize];
        let len = n.link_len as usize;
        let mut out = [EMPTY_LINK; MAX_LINK_COUNT];
        if len > 0 {
            out[0] = n.first;
            if len > 1 {
                let ex = &self.extra_links[n.extra_links as usize];
                out[1..len].copy_from_slice(&ex[..len - 1]);
            }
        }
        (out, len)
    }

    pub fn clear(&mut self) {
        self.heads.clear();
        self.heads.push(Head {
            node: self.base,
            status: Status::Active,
            node_count_at_last_error: 0,
            summary: None,
            paused_lookahead: NONE,
            ext: 0,
        });
    }

    pub fn version_count(&self) -> usize {
        self.heads.len()
    }
    pub fn state(&self, v: usize) -> u16 {
        self.nodes[self.heads[v].node as usize].state
    }
    pub fn position(&self, v: usize) -> Len {
        self.nodes[self.heads[v].node as usize].pos
    }
    pub fn is_active(&self, v: usize) -> bool {
        self.heads[v].status == Status::Active
    }
    pub fn is_halted(&self, v: usize) -> bool {
        self.heads[v].status == Status::Halted
    }
    pub fn is_paused(&self, v: usize) -> bool {
        self.heads[v].status == Status::Paused
    }
    pub fn halt(&mut self, v: usize) {
        self.heads[v].status = Status::Halted;
    }
    pub fn dynamic_precedence(&self, v: usize) -> i32 {
        self.nodes[self.heads[v].node as usize].dprec
    }
    pub fn halted_count(&self) -> usize {
        self.heads
            .iter()
            .filter(|h| h.status == Status::Halted)
            .count()
    }

    pub fn error_cost(&self, v: usize) -> u32 {
        let h = &self.heads[v];
        let n = &self.nodes[h.node as usize];
        let mut r = n.error_cost;
        if h.status == Status::Paused
            || (n.state == ERROR_STATE && n.link_len > 0 && n.first.tree == NONE)
        {
            r += crate::tree::ERROR_COST_PER_RECOVERY;
        }
        r
    }

    pub fn node_count_since_error(&mut self, v: usize) -> u32 {
        let nc = self.nodes[self.heads[v].node as usize].node_count;
        let h = &mut self.heads[v];
        if nc < h.node_count_at_last_error {
            h.node_count_at_last_error = nc;
        }
        nc - h.node_count_at_last_error
    }

    pub fn push(&mut self, a: &Arena, v: usize, tree: u32, pending: bool, state: u16) {
        let prev = self.heads[v].node;
        let p = &self.nodes[prev as usize];
        let mut n = SNode {
            state,
            link_len: 1,
            pos: p.pos,
            first: Link {
                node: prev,
                tree,
                pending,
            },
            extra_links: NONE,
            error_cost: p.error_cost,
            node_count: p.node_count,
            dprec: p.dprec,
            depth: p.depth + 1,
        };
        if tree != NONE {
            let t = a.n(tree);
            n.error_cost = n.error_cost.wrapping_add(t.cost());
            n.pos = n.pos.add(t.total());
            n.node_count += subtree_node_count(a, tree);
            n.dprec += t.dprec;
            if t.ext != NONE {
                self.heads[v].ext = t.ext;
            }
        }
        self.nodes.push(n);
        let id = (self.nodes.len() - 1) as u32;
        let h = &mut self.heads[v];
        h.node = id;
        if tree == NONE {
            h.node_count_at_last_error = self.nodes[id as usize].node_count;
        }
    }

    /// 官方 `stack_node_add_link`（改成工作表迭代，避免深递归）。
    fn add_link(&mut self, a: &Arena, target: u32, link: Link) {
        let mut work = vec![(target, link)];
        while let Some((me, link)) = work.pop() {
            if link.node == me {
                continue;
            }
            let mut handled = false;
            let lcnt = self.link_count(me);
            for i in 0..lcnt {
                let ex = self.link_at(me, i);
                if equivalent(a, ex.tree, link.tree) {
                    if ex.node == link.node {
                        if link.tree != NONE
                            && ex.tree != NONE
                            && a.n(link.tree).dprec > a.n(ex.tree).dprec
                        {
                            self.set_link(me, i, link);
                            let d = self.nodes[link.node as usize].dprec + a.n(link.tree).dprec;
                            self.nodes[me as usize].dprec = d;
                        }
                        handled = true;
                        break;
                    }
                    let (en, ln) = (
                        &self.nodes[ex.node as usize],
                        &self.nodes[link.node as usize],
                    );
                    if en.state == ln.state
                        && en.pos.bytes == ln.pos.bytes
                        && en.error_cost == ln.error_cost
                    {
                        let mut dp = ln.dprec;
                        let (sub, slen) = self.copy_links(link.node);
                        if link.tree != NONE {
                            dp += a.n(link.tree).dprec;
                        }
                        for l in &sub[..slen] {
                            work.push((ex.node, *l));
                        }
                        if dp > self.nodes[me as usize].dprec {
                            self.nodes[me as usize].dprec = dp;
                        }
                        handled = true;
                        break;
                    }
                }
            }
            if handled || self.link_count(me) == MAX_LINK_COUNT {
                continue;
            }
            let mut nc = self.nodes[link.node as usize].node_count;
            let mut dp = self.nodes[link.node as usize].dprec;
            if link.tree != NONE {
                nc += subtree_node_count(a, link.tree);
                dp += a.n(link.tree).dprec;
            }
            self.push_node_link(me, link);
            let m = &mut self.nodes[me as usize];
            if nc > m.node_count {
                m.node_count = nc;
            }
            if dp > m.dprec {
                m.dprec = dp;
            }
        }
    }

    pub fn can_merge(&self, v1: usize, v2: usize) -> bool {
        let (h1, h2) = (&self.heads[v1], &self.heads[v2]);
        let (n1, n2) = (&self.nodes[h1.node as usize], &self.nodes[h2.node as usize]);
        h1.status == Status::Active
            && h2.status == Status::Active
            && n1.state == n2.state
            && n1.pos.bytes == n2.pos.bytes
            && n1.error_cost == n2.error_cost
            && h1.ext == h2.ext
    }

    pub fn merge(&mut self, a: &Arena, v1: usize, v2: usize) -> bool {
        if !self.can_merge(v1, v2) {
            return false;
        }
        let (n1, n2) = (self.heads[v1].node, self.heads[v2].node);
        let (links, len) = self.copy_links(n2);
        for &l in &links[..len] {
            self.add_link(a, n1, l);
        }
        if self.nodes[n1 as usize].state == ERROR_STATE {
            self.heads[v1].node_count_at_last_error = self.nodes[n1 as usize].node_count;
        }
        self.remove_version(v2);
        true
    }

    pub fn remove_version(&mut self, v: usize) {
        self.heads.remove(v);
    }

    pub fn renumber_version(&mut self, v1: usize, v2: usize) {
        if v1 == v2 {
            return;
        }
        let mut src = self.heads.remove(v1);
        if self.heads[v2].summary.is_some() && src.summary.is_none() {
            src.summary = self.heads[v2].summary.take();
        }
        self.heads[v2] = src;
    }

    pub fn swap_versions(&mut self, v1: usize, v2: usize) {
        self.heads.swap(v1, v2);
    }

    pub fn copy_version(&mut self, v: usize) -> usize {
        let mut h = self.heads[v].clone();
        h.summary = None;
        self.heads.push(h);
        self.heads.len() - 1
    }

    pub fn pause(&mut self, v: usize, lookahead: u32) {
        let nc = self.nodes[self.heads[v].node as usize].node_count;
        let h = &mut self.heads[v];
        h.status = Status::Paused;
        h.paused_lookahead = lookahead;
        h.node_count_at_last_error = nc;
    }

    pub fn resume(&mut self, v: usize) -> u32 {
        let h = &mut self.heads[v];
        h.status = Status::Active;
        std::mem::replace(&mut h.paused_lookahead, NONE)
    }

    pub fn has_advanced_since_error(&self, a: &Arena, v: usize) -> bool {
        let h = &self.heads[v];
        let mut node = h.node;
        if self.nodes[node as usize].error_cost == 0 {
            return true;
        }
        loop {
            let n = &self.nodes[node as usize];
            if n.link_len > 0 && n.first.tree != NONE {
                let l = n.first;
                let t = a.n(l.tree);
                if t.total().bytes > 0 {
                    return true;
                } else if n.node_count > h.node_count_at_last_error && t.cost() == 0 {
                    node = l.node;
                    continue;
                }
            }
            return false;
        }
    }

    /// 通用遍历，回调返回 (弹出, 停止)。`goal >= 0` 时收集子树。
    fn iter(
        &mut self,
        a: &Arena,
        version: usize,
        goal: i32,
        mut cb: impl FnMut(&Iter, &SNode) -> (bool, bool),
    ) -> Vec<Slice> {
        let mut slices: Vec<Slice> = Vec::new();
        let include = goal >= 0;
        let mut its = vec![Iter {
            node: self.heads[version].node,
            trees: Vec::new(),
            count: 0,
            pending: true,
        }];
        while !its.is_empty() {
            let mut i = 0usize;
            let mut size = its.len();
            while i < size {
                let node_id = its[i].node;
                let (pop, stop0) = cb(&its[i], &self.nodes[node_id as usize]);
                let link_count = self.link_count(node_id);
                let stop = stop0 || link_count == 0;
                if pop {
                    let mut trees = if stop {
                        std::mem::take(&mut its[i].trees)
                    } else {
                        its[i].trees.clone()
                    };
                    trees.reverse();
                    self.add_slice(version, node_id, trees, &mut slices);
                }
                if stop {
                    its.remove(i);
                    size -= 1;
                    continue;
                }
                for j in 1..=link_count {
                    let (link, idx);
                    if j == link_count {
                        link = self.link_at(node_id, 0);
                        idx = i;
                    } else {
                        if its.len() >= MAX_ITERATOR_COUNT {
                            continue;
                        }
                        link = self.link_at(node_id, j);
                        let c = Iter {
                            node: its[i].node,
                            trees: its[i].trees.clone(),
                            count: its[i].count,
                            pending: its[i].pending,
                        };
                        its.push(c);
                        idx = its.len() - 1;
                    }
                    let it = &mut its[idx];
                    it.node = link.node;
                    if link.tree != NONE {
                        if include {
                            it.trees.push(link.tree);
                        }
                        if !a.n(link.tree).extra() {
                            it.count += 1;
                            if !link.pending {
                                it.pending = false;
                            }
                        }
                    } else {
                        it.count += 1;
                        it.pending = false;
                    }
                }
                i += 1;
            }
        }
        slices
    }

    fn add_slice(&mut self, original: usize, node: u32, trees: Vec<u32>, slices: &mut Vec<Slice>) {
        for i in (0..slices.len()).rev() {
            let v = slices[i].version;
            if self.heads[v].node == node {
                slices.insert(i + 1, Slice { version: v, trees });
                return;
            }
        }
        let o = &self.heads[original];
        let head = Head {
            node,
            status: Status::Active,
            node_count_at_last_error: o.node_count_at_last_error,
            summary: None,
            paused_lookahead: NONE,
            ext: o.ext,
        };
        self.heads.push(head);
        slices.push(Slice {
            version: self.heads.len() - 1,
            trees,
        });
    }

    /// 无分叉单链快路径：沿唯一前驱走 `count` 步并将子树逆序写入可复用 `trees`，遇到分叉返回 `None`。
    pub fn pop_count_fast(
        &mut self,
        a: &Arena,
        v: usize,
        count: u32,
        trees: &mut Vec<u32>,
    ) -> Option<usize> {
        trees.clear();
        let mut node_id = self.heads[v].node;
        let mut c = 0u32;
        while c < count {
            let n = &self.nodes[node_id as usize];
            if n.link_len != 1 {
                return None;
            }
            let l = n.first;
            node_id = l.node;
            if l.tree != NONE {
                trees.push(l.tree);
                if !a.n(l.tree).extra() {
                    c += 1;
                }
            } else {
                c += 1;
            }
        }
        trees.reverse();
        let o = &self.heads[v];
        self.heads.push(Head {
            node: node_id,
            status: Status::Active,
            node_count_at_last_error: o.node_count_at_last_error,
            summary: None,
            paused_lookahead: NONE,
            ext: o.ext,
        });
        Some(self.heads.len() - 1)
    }

    pub fn pop_count(&mut self, a: &Arena, v: usize, count: u32) -> Vec<Slice> {
        self.iter(a, v, count as i32, |it, _| {
            if it.count == count {
                (true, true)
            } else {
                (false, false)
            }
        })
    }

    pub fn pop_all(&mut self, a: &Arena, v: usize) -> Vec<Slice> {
        self.iter(a, v, 0, |_, n| (n.link_len == 0, false))
    }

    pub fn pop_error(&mut self, a: &Arena, v: usize) -> Vec<u32> {
        let node = self.heads[v].node;
        let lcnt = self.link_count(node);
        let mut has = false;
        for i in 0..lcnt {
            let l = self.link_at(node, i);
            if l.tree != NONE && a.n(l.tree).is_error() {
                has = true;
                break;
            }
        }
        if !has {
            return Vec::new();
        }
        let mut found = false;
        let mut pop = self.iter(a, v, 1, |it, _| {
            if !it.trees.is_empty() {
                if !found && a.n(it.trees[0]).is_error() {
                    found = true;
                    (true, true)
                } else {
                    (false, true)
                }
            } else {
                (false, false)
            }
        });
        if pop.is_empty() {
            return Vec::new();
        }
        let s = pop.remove(0);
        self.renumber_version(s.version, v);
        s.trees
    }

    pub fn record_summary(&mut self, a: &Arena, v: usize, max_depth: u32) {
        let mut summary: Vec<SumEntry> = Vec::new();
        let mut cb_summary = std::mem::take(&mut summary);
        self.iter(a, v, -1, |it, n| {
            let depth = it.count;
            if depth > max_depth {
                return (false, true);
            }
            for e in cb_summary.iter().rev() {
                if e.depth < depth {
                    break;
                }
                if e.depth == depth && e.state == n.state {
                    return (false, false);
                }
            }
            cb_summary.push(SumEntry {
                pos: n.pos,
                depth,
                state: n.state,
            });
            (false, false)
        });
        self.heads[v].summary = Some(cb_summary);
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}
