#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

use std::str;

type Tbitmap = u16;

#[derive(Copy, Clone, Debug)]
struct FlagsIndex(u32);

impl FlagsIndex {
    #[inline]
    fn new(flags: u8, index: usize) -> FlagsIndex {
        debug_assert!(flags & 0x3 == flags);
        FlagsIndex(((index as u32) << 2) | (flags as u32))
    }

    #[inline]
    fn flags_get(&self) -> u8 {
        (self.0 & 0x3) as u8
    }

    #[inline]
    fn index_get(&self) -> usize {
        (self.0 >> 2) as usize
    }
}

#[derive(Clone, Debug)]
struct Tleaf<TK: Clone + Eq + PartialEq + AsRef<[u8]>> {
    key: TK,
    val: Vec<u8>,
}

#[derive(Clone, Debug)]
struct Tbranch<TK: Clone + Eq + PartialEq + AsRef<[u8]>> {
    twigs: Vec<Node<TK>>,
    flags_index: FlagsIndex,
    bitmap: Tbitmap,
}

#[derive(Clone, Debug)]
enum Node<TK: Clone + Eq + PartialEq + AsRef<[u8]>> {
    Leaf(Tleaf<TK>),
    Branch(Tbranch<TK>),
}

#[derive(Default, Debug)]
pub struct Trie<TK: Clone + Eq + PartialEq + AsRef<[u8]>> {
    root: Option<Node<TK>>,
}

impl<TK: Clone + Eq + PartialEq + AsRef<[u8]>> Tbranch<TK> {
    #[inline]
    fn twigoff(&self, b: Tbitmap) -> usize {
        (self.bitmap & (b - 1)).count_ones() as usize
    }
}

impl<TK: Clone + Eq + PartialEq + AsRef<[u8]>> Node<TK> {
    #[inline]
    fn flags_index_get(&self) -> (u8, usize) {
        let branch = match *self {
            Node::Branch(ref branch) => branch,
            _ => unreachable!(),
        };
        let flags_index = branch.flags_index;
        (flags_index.flags_get(), flags_index.index_get())
    }

    #[inline]
    fn is_branch(&self) -> bool {
        match *self {
            Node::Leaf(_) => false,
            Node::Branch(ref branch) => {
                debug_assert_eq!((branch.flags_index.flags_get() & 1), 1);
                true
            }
        }
    }

    #[inline]
    fn twigbit(&self, key: &[u8]) -> Tbitmap {
        let len = key.len() - 1;
        let (flags, index) = self.flags_index_get();
        let i = index as usize;
        if i >= len {
            return 1;
        }
        Node::<TK>::nibbit(key[i], flags)
    }

    #[inline]
    fn has_twig(&self, bit: Tbitmap) -> bool {
        let branch = match *self {
            Node::Branch(ref branch) => branch,
            _ => unreachable!(),
        };
        (branch.bitmap & bit) != 0
    }

    #[inline]
    fn twigoff(&self, b: Tbitmap) -> usize {
        match *self {
            Node::Branch(ref branch) => branch.twigoff(b),
            _ => unreachable!(),
        }
    }

    #[inline]
    fn twig(&self, i: usize) -> &Node<TK> {
        let branch = match *self {
            Node::Branch(ref branch) => branch,
            _ => unreachable!(),
        };
        &branch.twigs[i]
    }

    #[inline]
    fn twig_mut(&mut self, i: usize) -> &mut Node<TK> {
        let branch = match *self {
            Node::Branch(ref mut branch) => branch,
            _ => unreachable!(),
        };
        &mut branch.twigs[i]
    }

    #[inline]
    fn twigoff_max(&self, b: Tbitmap) -> (usize, usize) {
        let branch = match *self {
            Node::Branch(ref branch) => branch,
            _ => unreachable!(),
        };
        let off = self.twigoff(b);
        let max = branch.bitmap.count_ones() as usize;
        (off, max)
    }

    #[inline]
    fn nibbit(k: u8, flags: u8) -> Tbitmap {
        let mask = ((flags.wrapping_sub(2)) ^ 0x0f) & 0xff;
        let shift = (2 - flags) << 2;
        (1 as Tbitmap) << ((k & mask) >> shift)
    }

    fn _next_ge<'s>(self: &'s Node<TK>, key: &[u8]) -> Option<(&'s TK, &'s Vec<u8>)> {
        if self.is_branch() {
            let (s, m) = self.twigoff_max(self.twigbit(key));
            for s in s..m {
                if let ret @ Some(_) = self.twig(s)._next_ge(key) {
                    return ret;
                }
            }
            return None;
        }
        let leaf = match *self {
            Node::Leaf(ref leaf) => leaf,
            _ => unreachable!(),
        };
        Some((&leaf.key, &leaf.val))
    }

    fn _next_gt<'s>(self: &'s Node<TK>, key: &[u8]) -> Option<(&'s TK, &'s Vec<u8>)> {
        if self.is_branch() {
            let (s, m) = self.twigoff_max(self.twigbit(key));
            for s in s..m {
                if let ret @ Some(_) = Self::_next_gt(self.twig(s), key) {
                    return ret;
                }
            }
            return None;
        }
        let leaf = match *self {
            Node::Leaf(ref leaf) => leaf,
            _ => unreachable!(),
        };
        if leaf.key.as_ref() == key {
            None
        } else {
            Some((&leaf.key, &leaf.val))
        }
    }
}

impl<TK: Clone + Eq + PartialEq + AsRef<[u8]>> Trie<TK> {
    pub fn get(&self, key: &TK) -> Option<&Vec<u8>> {
        let key = key.as_ref();
        if self.root.is_none() {
            return None;
        }
        let len = match key.len() {
            0 => return None,
            len => len - 1,
        };
        assert_eq!(key[len], 0);
        let mut t = self.root.as_ref().unwrap();
        while t.is_branch() {
            let b = t.twigbit(key);
            if !t.has_twig(b) {
                return None;
            }
            t = t.twig(t.twigoff(b));
        }
        let leaf = match *t {
            Node::Leaf(ref leaf) => leaf,
            _ => unreachable!(),
        };
        if leaf.key.as_ref() != key {
            return None;
        }
        Some(&leaf.val)
    }

    pub fn set(&mut self, key: TK, val: Vec<u8>) -> bool {
        let len = match key.as_ref().len() {
            0 => panic!("key cannot be empty"),
            len if len >= 0xffffff => panic!("key is too long"),
            len => len - 1,
        };
        if key.as_ref()[len] != 0 {
            panic!("key must be zero-terminated")
        }
        if self.root.is_none() {
            let new_node = Node::Leaf(Tleaf {
                key: key,
                val: val,
            });
            self.root = Some(new_node);
            return true;
        }
        let mut t: *mut Node<TK> = self.root.as_mut().unwrap();
        let t = unsafe {
            while (&*t).is_branch() {
                let b = (&*t).twigbit(&key.as_ref());
                let i = if (&*t).has_twig(b) {
                    (&*t).twigoff(b)
                } else {
                    0
                };
                t = (&mut *t).twig_mut(i);
            }
            &mut *t
        };
        let leaf = match *t {
            Node::Leaf(ref mut leaf) => leaf,
            _ => unreachable!(),
        };
        let leaf_key = &leaf.key;
        let mut i = 0;
        let mut x = 0;
        while i <= len {
            x = key.as_ref()[i] ^ leaf_key.as_ref()[i];
            if x != 0 {
                break;
            }
            i += 1;
        }
        if x == 0 {
            leaf.val = val;
            return false;
        }
        let k1 = key.as_ref()[i];
        let k2 = leaf_key.as_ref()[i];
        let f = if (x & 0xf0) != 0 { 1 } else { 2 };
        let mut t: *mut Node<TK> = self.root.as_mut().unwrap();
        let (t, grow_branch) = unsafe {
            let mut grow_branch = false;
            while (&*t).is_branch() {
                let (flags, index) = (&*t).flags_index_get();
                if i == index && f == flags {
                    grow_branch = true;
                    break;
                }
                if (i == index && f < flags) || i < index {
                    break;
                }
                let b = (&*t).twigbit(&key.as_ref());
                debug_assert!((&*t).has_twig(b));
                t = (&mut *t).twig_mut((&*t).twigoff(b));
            }
            (&mut *t, grow_branch)
        };
        let new_node = Node::Leaf(Tleaf {
            key: key,
            val: val,
        });
        let b1 = Node::<TK>::nibbit(k1, f);
        if grow_branch {
            Self::_grow_branch(t, b1, new_node);
        } else {
            let b2 = Node::<TK>::nibbit(k2, f);
            Self::_new_branch(t, b1, b2, f, i, new_node);
        }
        true
    }

    fn _new_branch(t: &mut Node<TK>,
                   b1: Tbitmap,
                   b2: Tbitmap,
                   f: u8,
                   i: usize,
                   new_node: Node<TK>) {
        let twigs: Vec<Node<TK>> = Vec::with_capacity(2);
        let mut new_t = Tbranch {
            twigs: twigs,
            flags_index: FlagsIndex::new(f, i),
            bitmap: b1 | b2,
        };
        if new_t.twigoff(b1) == 0 {
            new_t.twigs.push(new_node);
            new_t.twigs.push(t.clone());
        } else {
            new_t.twigs.push(t.clone());
            new_t.twigs.push(new_node);
        }
        *t = Node::Branch(new_t);
    }

    fn _grow_branch(t: &mut Node<TK>, b1: Tbitmap, new_node: Node<TK>) {
        debug_assert!(!t.has_twig(b1));
        let branch = match *t {
            Node::Branch(ref mut branch) => branch,
            _ => unreachable!(),
        };
        let s = branch.twigoff(b1);
        branch.twigs.insert(s, new_node);
        branch.bitmap |= b1;
    }

    pub fn del(&mut self, key: &TK) -> bool {
        if self.root.is_none() {
            return false;
        }
        let len = match key.as_ref().len() {
            0 => return false,
            len => len - 1,
        };
        assert_eq!(key.as_ref()[len], 0);
        let mut t: *mut Node<TK> = self.root.as_mut().unwrap();
        let (t, p, b): (&mut Node<TK>, _, _) = unsafe {
            let mut b = 0;
            let mut p = None;
            while (&*t).is_branch() {
                b = (&*t).twigbit(key.as_ref());
                if !(&*t).has_twig(b) {
                    return false;
                }
                p = Some(t);
                t = (&mut *t).twig_mut((&*t).twigoff(b));
            }
            (&mut *t, p, b)
        };
        let leaf = match *t {
            Node::Leaf(ref leaf) => leaf,
            _ => unreachable!(),
        };
        if leaf.key != *key {
            return false;
        }
        let t: &mut Node<TK> = match p {
            None => {
                self.root = None;
                return true;
            }
            Some(t) => unsafe { &mut *t },
        };
        let (s, m) = t.twigoff_max(b);
        if m == 2 {
            let t2 = t.twig(1 - s).clone();
            *t = t2;
        } else {
            let branch = match *t {
                Node::Branch(ref mut branch) => branch,
                _ => unreachable!(),
            };
            branch.twigs.remove(s);
            branch.twigs.shrink_to_fit();
            branch.bitmap &= !b;
        }
        true
    }

    pub fn prefix_iter<'s>(&'s self, key: &'s TK) -> TrieIterator<TK> {
        TrieIterator {
            t: self.root.as_ref().unwrap(),
            key: key.as_ref(),
            gt: false,
        }
    }
}

pub struct TrieIterator<'s, TK: 's + Clone + Eq + PartialEq + AsRef<[u8]>> {
    t: &'s Node<TK>,
    key: &'s [u8],
    gt: bool,
}

impl<'s, TK: Clone + Eq + PartialEq + AsRef<[u8]>> TrieIterator<'s, TK> {
    #[inline]
    pub fn different(mut self) -> Self {
        self.gt = true;
        self
    }
}

impl<'s, TK: Clone + Eq + PartialEq + AsRef<[u8]>> Iterator for TrieIterator<'s, TK> {
    type Item = (&'s TK, &'s [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        let res = if self.gt {
            self.t._next_gt(self.key)
        } else {
            self.t._next_ge(self.key)
        };
        match res {
            None => None,
            Some((key, value)) => {
                self.key = key.as_ref();
                self.gt = true;
                return Some((key, value));
            }
        }
    }
}
