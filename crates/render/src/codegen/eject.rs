//! Eject: one generated node's function turned into editable text among its generated peers, and back.
//!
//! An [`Ejected`] holds the occupant as generated ([`Generated`]) and, for each ejected node, the text now standing for
//! its function. The function boundaries are the generator's (`node_<id>`, [`super::function_name`]), so splicing is
//! replacing one function's text and nothing else: every peer stays byte for byte, and reverting a node, or all of
//! them (the whole slot's eject is every node ejected), gives the generated source back byte for byte.
//!
//! An ejected function keeps its `// @uniform` lines, so its controls stay live; the author's edit to those lines is a
//! declaration edit like any other ([`crate::custom::reconcile`]).
//!
//! **Dependencies.** A function's text may call other nodes' functions, `node_<n>(`. A regenerated tree
//! ([`Ejected::regenerate`]) renews every peer and leaves an ejected function as its author has it; it reports where
//! the two no longer agree, never silently: a node gone from the tree whose function is ejected, a call to a function
//! no node has, and an ejected function whose calls differ from the regenerated one's (the tree's wiring changed).

use std::collections::BTreeMap;

use super::{function_name, Generated};

/// Why an eject, edit or revert was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EjectError(pub String);

impl std::fmt::Display for EjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for EjectError {}

/// Where an ejected function and the regenerated tree disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Drift {
    /// The tree has no node `id` now: its ejected text is dropped.
    Gone { id: u32 },
    /// Node `id`'s text calls `callee`, which no node's function is.
    Missing { id: u32, callee: u32 },
    /// Node `id`'s text calls other functions than the regenerated one does: the wiring moved under it.
    Rewired {
        id: u32,
        ejected: Vec<u32>,
        generated: Vec<u32>,
    },
}

/// A generated occupant with some nodes' functions ejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ejected {
    generated: Generated,
    text: BTreeMap<u32, String>,
}

impl Ejected {
    /// `generated`, nothing ejected.
    pub fn new(generated: Generated) -> Ejected {
        Ejected {
            generated,
            text: BTreeMap::new(),
        }
    }

    /// Node `id`'s function as generated.
    fn function(&self, id: u32) -> Result<&str, EjectError> {
        self.generated
            .functions
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.text.as_str())
            .ok_or_else(|| EjectError(format!("no node {id}")))
    }

    /// Ejects node `id`: its function is now text, as generated, for the author to edit. Returns that text. Ejecting an
    /// ejected node returns its text as it stands.
    pub fn eject(&mut self, id: u32) -> Result<&str, EjectError> {
        if !self.text.contains_key(&id) {
            let generated = self.function(id)?.to_owned();
            self.text.insert(id, generated);
        }
        Ok(&self.text[&id])
    }

    /// Ejects every node: the whole slot as text.
    pub fn eject_all(&mut self) {
        for f in &self.generated.functions {
            self.text.entry(f.id).or_insert_with(|| f.text.clone());
        }
    }

    /// Replaces ejected node `id`'s text with `text`, which must define `fn node_<id>`, so the boundary the splice
    /// and its peers rely on stays.
    pub fn edit(&mut self, id: u32, text: &str) -> Result<(), EjectError> {
        let Some(held) = self.text.get_mut(&id) else {
            return Err(EjectError(format!("node {id} is not ejected")));
        };
        let name = function_name(id);
        if !defines(text, &name) {
            return Err(EjectError(format!(
                "node {id}'s text must define `fn {name}`: its peers call it by that name"
            )));
        }
        *held = text.to_owned();
        Ok(())
    }

    /// Puts node `id` back to its generated function.
    pub fn revert(&mut self, id: u32) -> Result<(), EjectError> {
        self.text
            .remove(&id)
            .map(|_| ())
            .ok_or_else(|| EjectError(format!("node {id} is not ejected")))
    }

    /// Puts every node back to its generated function.
    pub fn revert_all(&mut self) {
        self.text.clear();
    }

    /// Whether node `id` is ejected.
    pub fn is_ejected(&self, id: u32) -> bool {
        self.text.contains_key(&id)
    }

    /// The ejected node ids.
    pub fn ejected(&self) -> impl Iterator<Item = u32> + '_ {
        self.text.keys().copied()
    }

    /// The occupant's text: the generated source with each ejected function's text in its place.
    pub fn source(&self) -> String {
        let mut source = self.generated.source.clone();
        for f in &self.generated.functions {
            if let Some(edited) = self.text.get(&f.id) {
                // A function's generated text carries its own name in its header, so it occurs once.
                source = source.replacen(&f.text, edited, 1);
            }
        }
        source
    }

    /// The ids of the node functions node `id`'s text calls: its ejected text, or as generated.
    pub fn dependencies(&self, id: u32) -> Result<Vec<u32>, EjectError> {
        let text = match self.text.get(&id) {
            Some(t) => t.as_str(),
            None => self.function(id)?,
        };
        Ok(calls(text, id))
    }

    /// The tree regenerated as `generated`: every peer takes its new function, every ejected one keeps its text.
    /// Returns where the ejected texts and the tree now disagree.
    pub fn regenerate(&mut self, generated: Generated) -> Vec<Drift> {
        self.generated = generated;
        let ids: Vec<u32> = self.generated.functions.iter().map(|f| f.id).collect();
        let mut drift = Vec::new();
        let held: Vec<u32> = self.text.keys().copied().collect();
        for id in held {
            if !ids.contains(&id) {
                self.text.remove(&id);
                drift.push(Drift::Gone { id });
                continue;
            }
            let ejected = calls(&self.text[&id], id);
            for &callee in &ejected {
                if !ids.contains(&callee) {
                    drift.push(Drift::Missing { id, callee });
                }
            }
            let fresh = self.function(id).map(|t| calls(t, id)).unwrap_or_default();
            if ejected != fresh {
                drift.push(Drift::Rewired {
                    id,
                    ejected,
                    generated: fresh,
                });
            }
        }
        drift
    }
}

/// Whether `text` defines `fn name(`.
fn defines(text: &str, name: &str) -> bool {
    let word = format!("fn {name}(");
    text.lines().any(|l| {
        l.trim_start().starts_with(&word) || l.trim_start().starts_with(&format!("fn {name} ("))
    })
}

/// The node ids `text` calls as `node_<n>(`, but its own `own`, sorted and distinct.
fn calls(text: &str, own: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let code: String = text
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let mut rest = code.as_str();
    while let Some(at) = rest.find("node_") {
        let before_ok = rest[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        let tail = &rest[at + 5..];
        let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
        let after = &tail[digits.len()..];
        if before_ok && !digits.is_empty() && after.trim_start().starts_with('(') {
            if let Ok(n) = digits.parse::<u32>() {
                if n != own && !out.contains(&n) {
                    out.push(n);
                }
            }
        }
        rest = &rest[at + 5..];
    }
    out.sort_unstable();
    out
}
