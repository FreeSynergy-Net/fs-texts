//! The flat block list as a tree of containers.
//!
//! Pattern: Composite (rebuilt) — blocks carry their containers outermost first; consecutive
//! blocks with the same instance at one depth form one group.

use fs_text::{Block, Container};

/// A leaf block or a group of nodes in one container.
pub(crate) enum Node<'a> {
    /// A block with no (more) container.
    Leaf(&'a Block),
    /// A container and what is in it.
    Group(&'a Container, Vec<Node<'a>>),
}

/// Build the tree of `blocks`.
pub(crate) fn build(blocks: &[Block]) -> Vec<Node<'_>> {
    let all: Vec<&Block> = blocks.iter().collect();
    level(&all, 0)
}

fn level<'a>(blocks: &[&'a Block], depth: usize) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < blocks.len() {
        let b = blocks[i];
        let Some(c) = b.attrs.containers.get(depth) else {
            out.push(Node::Leaf(b));
            i += 1;
            continue;
        };
        let mut j = i + 1;
        while j < blocks.len()
            && blocks[j].attrs.containers.get(depth).is_some_and(|d| {
                d.inst() == c.inst() && std::mem::discriminant(d) == std::mem::discriminant(c)
            })
        {
            j += 1;
        }
        out.push(Node::Group(c, level(&blocks[i..j], depth + 1)));
        i = j;
    }
    out
}

/// Labels of all footnote definitions anywhere in the tree.
pub(crate) fn footnote_labels(nodes: &[Node<'_>], out: &mut Vec<String>) {
    for n in nodes {
        if let Node::Group(c, kids) = n {
            if let Container::Footnote { label, .. } = c {
                out.push(label.clone());
            }
            footnote_labels(kids, out);
        }
    }
}
