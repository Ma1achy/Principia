//! The kernel's stages: [`decode`], the decode stage, [`canon`], canonicalisation `C` as a stage of its own, and
//! [`encode`], encode `E` and its `β` as entry points of their own, and [`fragment`], the decode, encode and shape map
//! the translation exports to the fragment. [`branch_read`] is no stage of the pipeline: it reads the branches
//! of canon and of each decode variant back for the cross-backend check, and [`parity`] one decode and one step on
//! shared states for the parity harness.

pub mod branch_read;
pub mod canon;
pub mod decode;
pub mod encode;
pub mod fragment;
pub mod parity;
