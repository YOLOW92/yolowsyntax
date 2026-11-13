//! 标记-整理回收：在解析主循环的安全点丢掉不可达的栈节点与子树，让峰值内存跟随“活的”部分而不是累计分配量。
//!
//! 栈节点每个词都会新增、大部分很快就不可达，所以单独频繁整理（代价只与活的栈成正比）；
//! 子树竞技场只在翻倍后才标记，而且活的比例太高时不重建（重建会让新旧两份同时在内存里）。

use crate::parser::Parser;
use crate::tree::{Arena, NONE};

const ST_MIN_TRIGGER: usize = 4096;
const MIN_TRIGGER: usize = 200_000;

impl Parser<'_> {
    /// 只能在没有悬空节点编号的时刻调用（主循环两轮之间）。
    pub fn maybe_compact(&mut self) {
        if self.st.nodes.len() > ST_MIN_TRIGGER.max(self.live_st_after.saturating_mul(2)) {
            self.compact_stack();
        }
        if self.a.nodes.len() > MIN_TRIGGER.max(self.live_after.saturating_mul(2)) {
            self.compact_arena();
        }
    }

    /// 回收不可达的栈节点，子树编号不变。基节点（0 号）始终保留。
    pub fn compact_stack(&mut self) {
        let sn = self.st.nodes.len();
        self.st.scratch_smap.clear();
        self.st.scratch_smap.resize(sn, NONE);
        let mut order: Vec<u32> = Vec::with_capacity(64);
        let mut work: Vec<u32> = Vec::with_capacity(64);
        work.push(0);
        work.extend(self.st.heads.iter().map(|h| h.node));
        while let Some(n) = work.pop() {
            if self.st.scratch_smap[n as usize] != NONE {
                continue;
            }
            self.st.scratch_smap[n as usize] = 0;
            order.push(n);
            let sn_node = self.st.nodes[n as usize];
            if sn_node.link_len > 0 {
                work.push(sn_node.first.node);
                if sn_node.link_len > 1 {
                    let ex = &self.st.extra_links[sn_node.extra_links as usize];
                    for l in &ex[..sn_node.link_len as usize - 1] {
                        work.push(l.node);
                    }
                }
            }
        }
        order.sort_unstable();
        for (i, &o) in order.iter().enumerate() {
            self.st.scratch_smap[o as usize] = i as u32;
        }
        self.st.scratch_nodes.clear();
        self.st.scratch_extra.clear();
        for &o in &order {
            let mut n = self.st.nodes[o as usize];
            if n.link_len > 0 {
                n.first.node = self.st.scratch_smap[n.first.node as usize];
                if n.link_len > 1 {
                    let mut ex = self.st.extra_links[n.extra_links as usize];
                    for l in &mut ex[..n.link_len as usize - 1] {
                        l.node = self.st.scratch_smap[l.node as usize];
                    }
                    n.extra_links = self.st.scratch_extra.len() as u32;
                    self.st.scratch_extra.push(ex);
                }
            }
            self.st.scratch_nodes.push(n);
        }
        std::mem::swap(&mut self.st.nodes, &mut self.st.scratch_nodes);
        std::mem::swap(&mut self.st.extra_links, &mut self.st.scratch_extra);
        for h in &mut self.st.heads {
            h.node = self.st.scratch_smap[h.node as usize];
        }
        self.live_st_after = self.st.nodes.len();
    }

    /// 回收不可达的子树；活的占九成以上就不重建。
    pub fn compact_arena(&mut self) {
        let an = self.a.nodes.len();
        let mut live = vec![false; an];
        let mut roots: Vec<u32> = Vec::new();
        for n in &self.st.nodes {
            if n.link_len > 0 {
                if n.first.tree != NONE {
                    roots.push(n.first.tree);
                }
                if n.link_len > 1 {
                    let ex = &self.st.extra_links[n.extra_links as usize];
                    for l in &ex[..n.link_len as usize - 1] {
                        if l.tree != NONE {
                            roots.push(l.tree);
                        }
                    }
                }
            }
        }
        roots.extend(
            self.st
                .heads
                .iter()
                .map(|h| h.paused_lookahead)
                .filter(|t| *t != NONE),
        );
        if self.finished != NONE {
            roots.push(self.finished);
        }
        let mut count = 0usize;
        while let Some(t) = roots.pop() {
            if live[t as usize] {
                continue;
            }
            live[t as usize] = true;
            count += 1;
            roots.extend_from_slice(self.a.children(t));
        }
        if count * 10 >= an * 9 {
            self.live_after = an;
            return;
        }
        let mut amap = vec![NONE; an];
        let mut next = 0u32;
        for (i, &l) in live.iter().enumerate() {
            if l {
                amap[i] = next;
                next += 1;
            }
        }
        let old = std::mem::take(&mut self.a);
        let mut old = old;
        let mut na = Arena {
            nodes: Vec::with_capacity(count),
            ..Default::default()
        };
        na.ext = std::mem::take(&mut old.ext);
        na.ext_map = std::mem::take(&mut old.ext_map);
        na.ext_total = old.ext_total;
        for (i, n) in old.nodes.iter().enumerate() {
            if !live[i] {
                continue;
            }
            let mut c = n.clone();
            if c.child_count > 0 {
                c.first_child = na.kids.len() as u32;
                na.kids
                    .extend(old.children(i as u32).iter().map(|&k| amap[k as usize]));
            }
            na.nodes.push(c);
        }
        drop(old);
        self.a = na;
        for n in &mut self.st.nodes {
            if n.link_len > 0 && n.first.tree != NONE {
                n.first.tree = amap[n.first.tree as usize];
            }
        }
        for ex in &mut self.st.extra_links {
            for l in ex {
                if l.tree != NONE {
                    l.tree = amap[l.tree as usize];
                }
            }
        }
        for h in &mut self.st.heads {
            if h.paused_lookahead != NONE {
                h.paused_lookahead = amap[h.paused_lookahead as usize];
            }
        }
        if self.finished != NONE {
            self.finished = amap[self.finished as usize];
        }
        self.live_after = self.a.nodes.len();
    }
}
