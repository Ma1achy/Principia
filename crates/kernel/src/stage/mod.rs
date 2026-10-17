//! The kernel's stages: [`decode`], the decode stage, [`canon`], canonicalisation `C` as a stage of its own, and
//! [`encode`], encode `E` and its `β` as entry points of their own.

pub mod canon;
pub mod decode;
pub mod encode;
