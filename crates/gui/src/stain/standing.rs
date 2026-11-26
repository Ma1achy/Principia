//! What the engine says of a node's controls (`engine::contract::stain::describe::standing`): which cannot change the
//! picture with the settings as they are, and the setting that would let each. A control that cannot is drawn dimmed,
//! with why in words ([`words`]), and stays a control: that is information, never a lock. A control the engine has not
//! answered for, or cannot judge, is drawn as any other.
//!
//! **Off the frame.** The first answer for a shader compiles it, so a thread of its own asks the engine ([`Standings`]),
//! for the selected node alone, and the answers are kept: they follow from the shader and its choices and toggles, so a
//! drag of any other control asks nothing.

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;

use engine::contract::render_state::{Node, NodeKind, Occupant};
use engine::contract::stain::describe::{self, Control, ControlView, Matters, Standing};

use super::Content;

/// How much of a dimmed control's opacity is left.
pub const DIMMED: f32 = 0.6;

/// The standing of `uniform` in `all`, where it cannot matter; none where it can, or nothing is known of it.
fn dead<'a>(all: &'a [Standing], uniform: &str) -> Option<&'a Matters> {
    let found = all.iter().find(|s| s.uniform == uniform);
    found.map(|s| &s.matters).filter(|m| **m != Matters::Now)
}

/// `m` in words.
fn line(m: &Matters) -> Option<String> {
    match m {
        Matters::Now => None,
        Matters::NotNow { needs: Some(n) } => Some(format!(
            "no effect now · matters when “{}” is {}",
            n.label, n.named
        )),
        Matters::NotNow { needs: None } => {
            Some("no effect now · no single setting would change that".to_owned())
        }
        Matters::Never => Some("no effect · its shader never uses it".to_owned()),
    }
}

/// Why none of `uniforms`, edited by one control, can matter, in words: the nearest to mattering speaks for it. None
/// where one of them can.
fn of<'a>(uniforms: impl Iterator<Item = &'a str>, all: &[Standing]) -> Option<String> {
    let nearest = |m: &Matters| match m {
        Matters::Now => 0,
        Matters::NotNow { needs: Some(_) } => 1,
        Matters::NotNow { needs: None } => 2,
        Matters::Never => 3,
    };
    let mut best: Option<&Matters> = None;
    for u in uniforms {
        let m = dead(all, u)?;
        if best.is_none_or(|b| nearest(m) < nearest(b)) {
            best = Some(m);
        }
    }
    line(best?)
}

/// Why control `c` cannot matter now, in words; none where it can, or nothing is known of it. A gradient can where one
/// of its stops can.
pub fn words(c: &ControlView, all: &[Standing]) -> Option<String> {
    let stops = c.stops.iter().map(|s| s.uniform.as_str());
    of(std::iter::once(c.uniform.as_str()).chain(stops), all)
}

/// [`words`] of one stop of a gradient; none where the whole gradient has them.
pub fn stop_words(c: &ControlView, uniform: &str, all: &[Standing]) -> Option<String> {
    match words(c, all) {
        Some(_) => None,
        None => of(std::iter::once(uniform), all),
    }
}

/// What a node's answer follows from: its kind, its shader, and its choices' and toggles' values, as bits.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    kind: NodeKind,
    occupant: Occupant,
    held: Vec<(String, Vec<u64>)>,
}

impl Key {
    /// `n`'s; none where its shader cannot be read.
    fn of(n: &Node) -> Option<Self> {
        let held = describe::describe_node(n)
            .ok()?
            .controls
            .into_iter()
            .filter(|c| matches!(c.control, Control::Choice(_) | Control::Toggle))
            .map(|c| {
                let bits = c.value.iter().map(|v| v.to_bits()).collect();
                (c.uniform, bits)
            })
            .collect();
        Some(Self {
            kind: n.kind,
            occupant: n.occupant.clone(),
            held,
        })
    }
}

/// What is asked of a node: the engine, but in a test.
pub type Judge = Arc<dyn Fn(&Node) -> Result<Vec<Standing>, String> + Send + Sync>;

type Answer = (Key, Arc<[Standing]>);

struct Worker {
    jobs: Sender<(Key, Node)>,
    done: Receiver<Answer>,
}

/// `judge`'s answer for `n`. An error is a shader that cannot be judged and a panic is caught: nothing is said of it.
fn run(judge: &Judge, n: &Node) -> Arc<[Standing]> {
    let judged = catch_unwind(AssertUnwindSafe(|| judge(n)));
    judged.ok().and_then(Result::ok).unwrap_or_default().into()
}

impl Worker {
    /// A thread asking `judge`; none where none starts.
    fn start(judge: Judge) -> Option<Self> {
        let (jobs, queued) = mpsc::channel::<(Key, Node)>();
        let (answer, done) = mpsc::channel();
        std::thread::Builder::new()
            .name("stain-standing".into())
            .spawn(move || {
                while let Ok(mut job) = queued.recv() {
                    // Only the newest is wanted.
                    while let Ok(newer) = queued.try_recv() {
                        job = newer;
                    }
                    let (key, n) = job;
                    let judged = run(&judge, &n);
                    if answer.send((key, judged)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Self { jobs, done })
    }
}

/// The most answers kept; past it they start again.
const KEPT_MOST: usize = 256;

/// The engine's answers, and the thread that asks for them.
pub struct Standings {
    judge: Judge,
    worker: Option<Worker>,
    asked: Option<Key>,
    /// The node last listed, and its key.
    seen: Option<(Node, Option<Key>)>,
    kept: HashMap<Key, Arc<[Standing]>>,
    #[cfg(test)]
    pub(crate) broken: Broken,
}

/// What a test breaks of [`Standings`].
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Broken {
    #[default]
    Nothing,
    /// The engine is asked on the caller's thread.
    Inline,
    /// No answer outlives the frame it is shown in.
    Forgets,
    /// A frame is asked for each time a kept answer is shown.
    Asks,
}

impl Default for Standings {
    fn default() -> Self {
        Self::with(Arc::new(|n| {
            describe::standing(n.kind, &n.occupant, &n.params).map_err(|why| why.to_string())
        }))
    }
}

impl Standings {
    /// The answers `judge` gives.
    pub fn with(judge: Judge) -> Self {
        Self {
            judge,
            worker: None,
            asked: None,
            seen: None,
            kept: HashMap::new(),
            #[cfg(test)]
            broken: Broken::default(),
        }
    }

    /// Whether an answer is still to come.
    pub fn pending(&self) -> bool {
        self.asked.is_some()
    }

    /// Takes what the thread has answered, without waiting.
    pub fn poll(&mut self) {
        while let Some(w) = &self.worker {
            match w.done.try_recv() {
                Ok(done) => self.take(done),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => self.stopped(),
            }
        }
    }

    /// Waits for the answer asked for: a frame with no window's, which draws the same whatever the thread's pace. A
    /// window's never waits.
    pub fn wait(&mut self) {
        while self.pending() {
            match self.worker.as_ref().map(|w| w.done.recv()) {
                Some(Ok(done)) => self.take(done),
                _ => self.stopped(),
            }
        }
    }

    fn take(&mut self, (key, judged): Answer) {
        self.asked.take_if(|asked| *asked == key);
        if self.kept.len() >= KEPT_MOST {
            self.kept.clear();
        }
        self.kept.insert(key, judged);
    }

    /// The thread ended without its answer: the next is asked of a new one.
    fn stopped(&mut self) {
        self.worker = None;
        self.asked = None;
    }

    /// Puts the answer for `n`, the selected node, in `c`, or asks for it: until it comes nothing is said of the node.
    pub fn list(&mut self, c: &mut Content, n: Option<&Node>) {
        self.poll();
        let Some(n) = n else {
            return;
        };
        // The key reads the shader's text, so it is made again only when the node changes.
        if self.seen.as_ref().is_none_or(|(of, _)| of != n) {
            self.seen = Some((n.clone(), Key::of(n)));
        }
        let Some((_, Some(key))) = &self.seen else {
            return;
        };
        let key = key.clone();
        #[cfg(test)]
        if self.broken == Broken::Inline {
            let judged = run(&self.judge, n);
            self.kept.insert(key.clone(), judged);
        }
        match self.kept.get(&key) {
            Some(judged) => {
                c.standing.insert(n.id, judged.clone());
                #[cfg(test)]
                if self.broken == Broken::Asks {
                    self.asked = Some(key);
                }
            }
            None => self.ask(key, n),
        }
        #[cfg(test)]
        if self.broken == Broken::Forgets {
            self.kept.clear();
        }
    }

    fn ask(&mut self, key: Key, n: &Node) {
        if self.asked.as_ref() == Some(&key) {
            return;
        }
        if self.worker.is_none() {
            self.worker = Worker::start(Arc::clone(&self.judge));
        }
        // With no thread (a browser has none) nothing is dimmed: asking here would stop the frame.
        self.asked = self
            .worker
            .as_ref()
            .and_then(|w| w.jobs.send((key.clone(), n.clone())).ok())
            .map(|()| key);
    }
}
