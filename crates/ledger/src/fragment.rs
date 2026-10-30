//! The fragment entry points' word layouts (TASK-M2-25; R-116; systems_architecture §7.1, the ledger as the layout
//! table): the words each of `kernel::stage::fragment`'s entry points reads its inputs from and writes its outputs to,
//! which `cargo xtask build-kernel`'s generated glue (`xtask::fragment_wgsl`) fills and reads in the fragment. One table
//! serves both sides: `cargo xtask codegen` writes the kernel's constants from it ([`crate::gen::fragment`],
//! `crates/kernel/src/generated/fragment_words.rs`), and the glue reads it here, so the two cannot disagree on an
//! offset.
//!
//! A layout is its parts in order, each a name and a length in 32-bit words; a part's offset is the sum of the lengths
//! before it, and the layout's size their sum. The lengths are the parts' own: `(u, v)` two f32; `Φ`'s `z` the eight
//! latent controls; the canonical `(m, r, p)` fifteen f32 (three masses, three positions and three momenta of two
//! components), in the decode stage's word order; the `ICDescriptor`'s twelve fields; `kernel::stage::encode`'s twelve
//! words (`z`, `α`, `β`, `λ` and the notices word); the shape map's `r` (six), `m` (three) and `n` (three); and the free
//! decode's latent `z` (eight), its `(m, r, p)` and its tag word (TASK-M2-26).

/// One part of a layout: its name, as the generated constant's suffix, and its length in words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Part {
    pub name: &'static str,
    pub len: usize,
}

/// One entry point's words: the generated constants' prefix and its parts in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub name: &'static str,
    pub parts: Vec<Part>,
}

impl Layout {
    /// The offset of part `name`, the sum of the lengths before it; `None` for a name not among the parts.
    pub fn offset(&self, name: &str) -> Option<usize> {
        let at = self.parts.iter().position(|p| p.name == name)?;
        Some(self.parts[..at].iter().map(|p| p.len).sum())
    }

    /// The offset of part `name`, panicking on a name not among the parts: for the glue, whose names are fixed.
    pub fn at(&self, name: &str) -> usize {
        self.offset(name)
            .unwrap_or_else(|| panic!("the {} words have no part {name}", self.name))
    }

    /// The layout's size in words.
    pub fn words(&self) -> usize {
        self.parts.iter().map(|p| p.len).sum()
    }
}

const fn part(name: &'static str, len: usize) -> Part {
    Part { name, len }
}

/// The decode's words: `(u, v)` in; `Φ`'s `z`, the latent flag, the canonical `(m, r, p)`, the descriptor and the tag
/// word out.
pub fn decode() -> Layout {
    Layout {
        name: "DECODE",
        parts: vec![
            part("UV", 2),
            part("Z", 8),
            part("Z_LATENT", 1),
            part("IC", 15),
            part("DESCRIPTOR", 12),
            part("TAG", 1),
        ],
    }
}

/// The descriptor's words: `(m, r, p)` in, the twelve fields out.
pub fn describe() -> Layout {
    Layout {
        name: "DESCRIBE",
        parts: vec![part("IC", 15), part("DESCRIPTOR", 12)],
    }
}

/// The encode's words: `(m, r, p)` in; `kernel::stage::encode`'s twelve words out.
pub fn encode() -> Layout {
    Layout {
        name: "ENCODE",
        parts: vec![part("IC", 15), part("OUT", 12)],
    }
}

/// The shape map's words: `r` and `m` in, `n` out.
pub fn shape() -> Layout {
    Layout {
        name: "SHAPE",
        parts: vec![part("R", 6), part("M", 3), part("N", 3)],
    }
}

/// The free decode's words: a latent `z` in; the canonical `(m, r, p)` and the tag word out (TASK-M2-26: the
/// ROUNDTRIP preset's second `D`, of the encode's `z`).
pub fn free() -> Layout {
    Layout {
        name: "FREE",
        parts: vec![part("Z", 8), part("IC", 15), part("TAG", 1)],
    }
}

/// Every layout, in the order the generated file lists them.
pub fn layouts() -> [Layout; 5] {
    [decode(), describe(), encode(), shape(), free()]
}
