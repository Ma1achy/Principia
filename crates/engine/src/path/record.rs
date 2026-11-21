//! A frame's record, by value (design § 17.5; export contract Part 3, "A morph's frames are recorded"): `(A, B, family,
//! s)`, `A` and `B` whole views, `s` as its f64 bits, and three optional fields, absent unless set: the alignment, the
//! retime table's hash and, under a lock, the anchor's words. Absent, the bytes are stage 1's. The easing is not in it:
//! timing changes no frame's content (Theorem 5).
//!
//! **The serialisation.** The plan is unclear here; chose JSON with every float written exactly (serde_json's
//! `float_roundtrip`) and the saved chart's words and `s` as IEEE bits in hex, not the canonical JCS text (R-309),
//! because JCS writes `−0` as `0` and a record must give back the bits the end rule compares.

use serde::{Deserialize, Serialize};

use kernel::chart::saved::{Saved, WORDS};

use super::{Family, KernelMap, Map, PathError, Pin, Segment, View};
use crate::contract::sim_config::SimConfig;

/// § 17.10's alignment `T`, in the window's similarity group: content, not timing. Typed, not built (stage 4).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Align {
    /// The rotation, in radians.
    pub rotation: f64,
    /// The offset, in the window's `[0,1]²` addresses.
    pub offset: [f64; 2],
}

/// The map as text: the saved chart by its words, in hex.
#[derive(Serialize, Deserialize)]
enum MapText {
    Latent,
    Kernel(KernelMap),
    Saved(Vec<String>),
}

/// A view as text.
#[derive(Serialize, Deserialize)]
struct ViewText {
    map: MapText,
    sim: SimConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pin: Option<Pin>,
}

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn unhex(s: &str) -> Result<f64, PathError> {
    u64::from_str_radix(s, 16)
        .map(f64::from_bits)
        .map_err(|e| PathError::Record(e.to_string()))
}

fn view_text(v: &View) -> ViewText {
    let map = match &v.map {
        Map::Latent => MapText::Latent,
        Map::Kernel(k) => MapText::Kernel(*k),
        Map::Saved(saved) => MapText::Saved(saved.to_words().iter().map(|w| hex(*w)).collect()),
    };
    ViewText {
        map,
        sim: v.sim.clone(),
        pin: v.pin,
    }
}

fn view_of(t: ViewText) -> Result<View, PathError> {
    let map = match t.map {
        MapText::Latent => Map::Latent,
        MapText::Kernel(k) => Map::Kernel(k),
        MapText::Saved(words) => {
            let words: Vec<f64> = words.iter().map(|w| unhex(w)).collect::<Result<_, _>>()?;
            let words: [f64; WORDS] = words
                .try_into()
                .map_err(|_| PathError::Record("a saved chart's word count".into()))?;
            Map::Saved(Box::new(Saved::from_words(&words).ok_or_else(|| {
                PathError::Record("a saved chart's words".into())
            })?))
        }
    };
    Ok(View {
        map,
        sim: t.sim,
        pin: t.pin,
    })
}

/// The view's bytes.
pub(super) fn view_bytes(v: &View) -> Vec<u8> {
    serde_json::to_vec(&view_text(v)).expect("a view serialises")
}

/// The record as text.
#[derive(Serialize, Deserialize)]
struct RecordText {
    a: ViewText,
    b: ViewText,
    family: Family,
    s: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    align: Option<Align>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    retime_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    anchor_words: Option<Vec<String>>,
}

/// A frame's record, by value.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameRecord {
    /// The view at `s = 0`.
    pub a: View,
    /// The view at `s = 1`.
    pub b: View,
    /// The family.
    pub family: Family,
    /// The path parameter's bits, `−0` canonicalised to `+0`.
    pub s_bits: u64,
    /// The alignment: not built (stage 4).
    pub align: Option<Align>,
    /// The retime table's hash: not built (stage 4).
    pub retime_hash: Option<u64>,
    /// Under a lock, the anchor's words, `(m, r, p)`: not built (stage 2).
    pub anchor_words: Option<[u64; 15]>,
}

impl FrameRecord {
    /// The record's bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let text = RecordText {
            a: view_text(&self.a),
            b: view_text(&self.b),
            family: self.family.clone(),
            s: format!("{:016x}", self.s_bits),
            align: self.align,
            retime_hash: self.retime_hash.map(|h| format!("{h:016x}")),
            anchor_words: self
                .anchor_words
                .map(|w| w.iter().map(|x| format!("{x:016x}")).collect()),
        };
        serde_json::to_vec(&text).expect("a record serialises")
    }

    /// The record `bytes` hold.
    pub fn read(bytes: &[u8]) -> Result<Self, PathError> {
        let t: RecordText =
            serde_json::from_slice(bytes).map_err(|e| PathError::Record(e.to_string()))?;
        let bits =
            |s: &str| u64::from_str_radix(s, 16).map_err(|e| PathError::Record(e.to_string()));
        let anchor_words = match t.anchor_words {
            None => None,
            Some(w) => Some(
                w.iter()
                    .map(|x| bits(x))
                    .collect::<Result<Vec<_>, _>>()?
                    .try_into()
                    .map_err(|_| PathError::Record("the anchor's word count".into()))?,
            ),
        };
        Ok(FrameRecord {
            a: view_of(t.a)?,
            b: view_of(t.b)?,
            family: t.family,
            s_bits: bits(&t.s)?,
            align: t.align,
            retime_hash: t.retime_hash.as_deref().map(bits).transpose()?,
            anchor_words,
        })
    }

    /// The frame the record names: its segment rebuilt from `A` and the family, refused ([`PathError::Rebuilt`]) where
    /// that does not reach the record's `B`, then its frame at `s`.
    pub fn frame(&self) -> Result<View, PathError> {
        if self.align.is_some() || self.retime_hash.is_some() || self.anchor_words.is_some() {
            return Err(PathError::NotBuilt(
                "alignment, retiming and the stored anchor",
            ));
        }
        Segment::rebuilt(&self.a, &self.b, &self.family)?.frame(f64::from_bits(self.s_bits))
    }
}
