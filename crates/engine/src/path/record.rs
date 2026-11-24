//! A frame's record, by value (design § 17.5; export contract Part 3, "A morph's frames are recorded"): `(A, B, family,
//! s)`, `A` and `B` whole views, `s` as its f64 bits, and three optional fields, absent unless set: the alignment, the
//! retime table's hash and, under a lock, the anchor's words. Absent, the bytes are stage 1's. The easing is not in it:
//! timing changes no frame's content (Theorem 5).
//!
//! **The serialisation.** The plan is unclear here; chose JSON with every float written exactly (serde_json's
//! `float_roundtrip`) and the saved chart's words and `s` as IEEE bits in hex, not the canonical JCS text, because
//! JCS writes `−0` as `0` and a record must give back the bits the end rule compares.

use serde::{Deserialize, Serialize};

use kernel::chart::saved::{Saved, WORDS};

use super::anchor::Anchor;
use super::blend::Blend;
use super::{BlendPath, CurvedTilt, Family, KernelMap, Map, PathError, Pin, Pivot, Segment, View};
use crate::contract::sim_config::SimConfig;

/// § 17.10's alignment `T`, in the window's similarity group: content, not timing. Typed, not built (stage 4).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Align {
    /// The rotation, in radians.
    pub rotation: f64,
    /// The offset, in the window's `[0,1]²` addresses.
    pub offset: [f64; 2],
}

/// The map as text: the saved chart by its words, in hex; a blend by its ends, its path, and its share and turn as
/// IEEE bits in hex.
#[derive(Serialize, Deserialize)]
enum MapText {
    Latent,
    Kernel(KernelMap),
    Saved(Vec<String>),
    Blend(Box<BlendText>),
}

/// A blend as text.
#[derive(Serialize, Deserialize)]
struct BlendText {
    a: ViewText,
    b: ViewText,
    path: BlendPath,
    s: String,
    turn: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    rehold: bool,
}

/// A view as text.
#[derive(Serialize, Deserialize)]
struct ViewText {
    map: MapText,
    sim: SimConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pin: Option<Pin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tilt: Option<CurvedTilt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pivot: Option<Pivot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    anchor: Option<AnchorText>,
}

/// The lock's anchor as text: its fifteen words in hex and the `assumed` label, written only where set.
#[derive(Serialize, Deserialize)]
struct AnchorText {
    words: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    assumed: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    slid: bool,
}

fn anchor_text(a: &Anchor) -> AnchorText {
    AnchorText {
        words: a.words.iter().map(|w| format!("{w:016x}")).collect(),
        assumed: a.assumed,
        slid: a.slid,
    }
}

fn anchor_of(t: AnchorText) -> Result<Anchor, PathError> {
    let words: Vec<u64> = t
        .words
        .iter()
        .map(|w| u64::from_str_radix(w, 16).map_err(|e| PathError::Record(e.to_string())))
        .collect::<Result<_, _>>()?;
    Ok(Anchor {
        words: words
            .try_into()
            .map_err(|_| PathError::Record("the anchor's word count".into()))?,
        assumed: t.assumed,
        slid: t.slid,
    })
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
        Map::Blend(blend) => MapText::Blend(Box::new(BlendText {
            a: view_text(&blend.a),
            b: view_text(&blend.b),
            path: blend.path,
            s: hex(blend.s()),
            turn: hex(blend.turn()),
            rehold: blend.rehold(),
        })),
    };
    ViewText {
        map,
        sim: v.sim.clone(),
        pin: v.pin,
        tilt: v.tilt,
        pivot: v.pivot,
        anchor: v.anchor.as_ref().map(anchor_text),
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
        MapText::Blend(text) => {
            let BlendText {
                a,
                b,
                path,
                s,
                turn,
                rehold,
            } = *text;
            let blend = Blend::new(view_of(a)?, view_of(b)?, path, unhex(&s)?)?;
            let blend = match unhex(&turn)? {
                turn if turn.to_bits() == 0 => blend,
                turn => blend.turned(turn)?,
            };
            let blend = blend.reholding(rehold);
            if blend.s().to_bits() != u64::from_str_radix(&s, 16).unwrap_or(u64::MAX) {
                return Err(PathError::Record("a blend's share is not canonical".into()));
            }
            Map::Blend(Box::new(blend))
        }
    };
    Ok(View {
        map,
        sim: t.sim,
        pin: t.pin,
        tilt: t.tilt,
        pivot: t.pivot,
        anchor: t.anchor.map(anchor_of).transpose()?,
    })
}

/// A record's version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    /// Stage 1's, written as no version at all, so its bytes are unchanged: no view carries a lock's record.
    One,
    /// Stage 2's, written `"version": 2`: every locked view carries its lock's record, the anchor.
    Two,
}

/// A stage-1 view converted: a locked leaf view gains its lock's record. A blend's ends are left as they are: stage 1's
/// blend was free whatever its ends' locks, and a lock's record on an end would make it a locked blend, another frame.
fn convert_view<L: super::blend::Leaves>(v: &View, leaves: &L) -> Result<View, PathError> {
    match &v.map {
        Map::Blend(_) => Ok(v.clone()),
        _ if v.sim.lock.locked && v.anchor.is_none() => super::anchor::record(v, leaves),
        _ => Ok(v.clone()),
    }
}

/// The view's bytes.
pub(super) fn view_bytes(v: &View) -> Vec<u8> {
    serde_json::to_vec(&view_text(v)).expect("a view serialises")
}

/// The record as text.
#[derive(Serialize, Deserialize)]
struct RecordText {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<u32>,
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
            version: (self.version() == Version::Two).then_some(2),
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
    ///
    /// A stage-1 record (no version) reads as it was written, a locked view in it with no lock's record; a stage-2
    /// record (version 2) must give every locked view its record; any other version is refused. A stage-1 record is
    /// converted on load by [`FrameRecord::read_converted`].
    pub fn read(bytes: &[u8]) -> Result<Self, PathError> {
        Ok(Self::parse(bytes)?.0)
    }

    /// The record `bytes`, of either version, a stage-1 record converted on load (§ 17.8; the saved-file rewrite of
    /// stage 2): each locked view at either end gains its lock's record, the pin's address and the anchor, the state
    /// the view shows there through `leaves` ([`super::anchor::record`]). A view with no lock, and a blend view, whose
    /// stage-1 frame was free, are unchanged; a stage-2 record reads as [`FrameRecord::read`] reads it.
    pub fn read_converted<L: super::blend::Leaves>(
        bytes: &[u8],
        leaves: &L,
    ) -> Result<Self, PathError> {
        let (record, version) = Self::parse(bytes)?;
        if version == Version::Two {
            return Ok(record);
        }
        let convert = |v: &View| convert_view(v, leaves);
        Ok(FrameRecord {
            a: convert(&record.a)?,
            b: convert(&record.b)?,
            ..record
        })
    }

    /// The views the record holds, a blend's ends included.
    fn views(&self) -> Vec<&View> {
        fn walk<'a>(v: &'a View, out: &mut Vec<&'a View>) {
            out.push(v);
            if let Map::Blend(blend) = &v.map {
                walk(&blend.a, out);
                walk(&blend.b, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.a, &mut out);
        walk(&self.b, &mut out);
        out
    }

    /// The record's version: stage 2's where any view in it carries a lock's record, else stage 1's, whose bytes are
    /// unchanged by stage 2.
    pub fn version(&self) -> Version {
        match self.views().iter().any(|v| v.anchor.is_some()) {
            true => Version::Two,
            false => Version::One,
        }
    }

    /// The record and its version, read from `bytes`: no version is stage 1's; stage 2's must give every locked view its
    /// lock's record; any other version is refused.
    fn parse(bytes: &[u8]) -> Result<(Self, Version), PathError> {
        let t: RecordText =
            serde_json::from_slice(bytes).map_err(|e| PathError::Record(e.to_string()))?;
        let version = match t.version {
            None => Version::One,
            Some(2) => Version::Two,
            Some(other) => {
                return Err(PathError::Record(format!(
                    "an unknown record version, {other}"
                )))
            }
        };
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
        let record = FrameRecord {
            a: view_of(t.a)?,
            b: view_of(t.b)?,
            family: t.family,
            s_bits: bits(&t.s)?,
            align: t.align,
            retime_hash: t.retime_hash.as_deref().map(bits).transpose()?,
            anchor_words,
        };
        let unrecorded = [&record.a, &record.b]
            .iter()
            .any(|v| v.sim.lock.locked && v.anchor.is_none() && !matches!(v.map, Map::Blend(_)));
        if version == Version::Two && unrecorded {
            return Err(PathError::Record(
                "a stage-2 record with a locked view and no lock's record".into(),
            ));
        }
        if version == Version::One && record.version() == Version::Two {
            return Err(PathError::Record(
                "a stage-1 record with a lock's record in it".into(),
            ));
        }
        Ok((record, version))
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
