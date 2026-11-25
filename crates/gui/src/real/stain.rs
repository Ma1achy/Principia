//! The real engine's stain content (render_gui_spec Part II §1): the store's own graph through the real assembler and
//! the registry. [`Compiler`] holds the stain as last compiled; a graph that differs from it only in params is the same
//! code, so it is never assembled again, and one that does not compile leaves the last that did in place.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::Arc;

use engine::contract::render_state::StainGraph;
use engine::contract::stain::assembled::{assemble, Assembled, Tier};

use crate::clock::time::Instant;
use crate::stain::catalogue::{catalogue, key};
use crate::stain::{described, node_code, Content};

/// The tier the real engine's records are stored at.
pub const TIER: Tier = engine::field::record::TIER;

/// What a compile did.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// The graph is with the worker; the last that compiled stays until it is done.
    Compiling,
    /// It assembled the graph, in so many ms.
    Assembled(f64),
    /// The graph is the one compiled, its params aside: nothing was assembled.
    Cached,
    /// The graph does not compile, and why: the last that did stays.
    Failed(String),
}

/// The graph a compile was asked for, and what came of it.
#[derive(Clone, Debug)]
struct Asked {
    graph: StainGraph,
    key: Option<u64>,
    /// The worker's job for it, while the outcome is [`Outcome::Compiling`].
    ticket: u64,
    outcome: Outcome,
}

/// What drawing the stain reported: the pipelines' errors, which name the node that keeps its last valid source, and
/// the notes on the records it is drawn from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawn {
    /// The fragment key of the graph last drawn, which the errors are of; none before the first draw.
    pub key: Option<u64>,
    /// Whether its pipeline is still compiling, the last valid picture drawn until it is done.
    pub compiling: bool,
    /// The errors.
    pub problems: Vec<String>,
    /// The notes.
    pub notes: Vec<String>,
}

/// What assembles a graph: the assembler, but in a test.
pub type Assembler = Arc<dyn Fn(&StainGraph) -> Result<Assembled, String> + Send + Sync>;

/// One graph for the worker.
struct Job {
    ticket: u64,
    graph: StainGraph,
    assembler: Assembler,
}

/// The worker's answer to a [`Job`]: the code and the ms it took, or why there is none.
struct Done {
    ticket: u64,
    result: Result<(Assembled, f64), String>,
}

impl Job {
    /// Runs it. A panic is caught: it fails its own graph, and the worker goes on to the newer ones.
    fn run(self) -> Done {
        let begun = Instant::now();
        let result = catch_unwind(AssertUnwindSafe(|| (self.assembler)(&self.graph)))
            .unwrap_or_else(|_| Err("the assembler panicked".to_owned()));
        Done {
            ticket: self.ticket,
            result: result.map(|code| (code, begun.elapsed().as_secs_f64() * 1e3)),
        }
    }
}

/// The thread that assembles. It ends when its [`Compiler`] is dropped, which does not wait for it, so closing the
/// window does not wait for an assembly: it holds no GPU object, whose teardown would have to be waited for.
struct Worker {
    jobs: Sender<Job>,
    done: Receiver<Done>,
}

impl Worker {
    /// A worker; none where no thread starts.
    fn start() -> Option<Self> {
        let (jobs, queued) = mpsc::channel::<Job>();
        let (answer, done) = mpsc::channel();
        std::thread::Builder::new()
            .name("stain-assemble".into())
            .spawn(move || {
                while let Ok(mut job) = queued.recv() {
                    // Only the newest is wanted.
                    while let Ok(newer) = queued.try_recv() {
                        job = newer;
                    }
                    if answer.send(job.run()).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Self { jobs, done })
    }
}

/// The stain as last compiled. A graph is assembled on a thread of its own, never on the caller's: assembling takes far
/// longer than a frame. The newest graph asked for is the only one whose result is taken.
pub struct Compiler {
    assembler: Assembler,
    /// The worker, started at the first graph it is needed for.
    worker: Option<Worker>,
    tickets: u64,
    asked: Option<Asked>,
    /// The code of the last graph that compiled, and the ms its assembly took.
    valid: Option<(Assembled, f64)>,
    /// The content as last made, and what it is of: the graph and what drawing it reported.
    content: Option<(StainGraph, Drawn, Content)>,
    /// The outcomes the worker has answered since they were last taken.
    finished: Vec<Outcome>,
    assemblies: u64,
    /// The control's: every result taken as it completes, whatever was asked since.
    #[cfg(test)]
    pub(crate) completion_order: bool,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::with(Arc::new(|g| {
            assemble(g, TIER).map_err(|why| why.to_string())
        }))
    }
}

impl Compiler {
    /// A compiler of nothing yet, its graphs assembled by `assembler`.
    pub fn with(assembler: Assembler) -> Self {
        Self {
            assembler,
            worker: None,
            tickets: 0,
            asked: None,
            valid: None,
            content: None,
            finished: Vec::new(),
            assemblies: 0,
            #[cfg(test)]
            completion_order: false,
        }
    }

    /// How many times a graph has been assembled.
    pub fn assemblies(&self) -> u64 {
        self.assemblies
    }

    /// Compiles `g`: hands it to the worker, unless the last graph asked for or the last that compiled has its code. It
    /// supersedes a graph still with the worker, whose result is then dropped.
    pub fn compile(&mut self, g: &StainGraph) -> Outcome {
        let wanted = key(g);
        self.content = None;
        self.tickets += 1;
        let mut asked = Asked {
            graph: g.clone(),
            key: wanted,
            ticket: self.tickets,
            outcome: Outcome::Compiling,
        };
        if let Some((_, ms)) = self.valid.as_ref().filter(|(c, _)| Some(c.key) == wanted) {
            asked.outcome = Outcome::Assembled(*ms);
            self.asked = Some(asked);
            return Outcome::Cached;
        }
        // The same code, with the worker or failed already: only the params differ.
        if let Some(a) = self
            .asked
            .as_mut()
            .filter(|a| wanted.is_some() && a.key == wanted)
        {
            a.graph = asked.graph;
            return a.outcome.clone();
        }
        let job = Job {
            ticket: asked.ticket,
            graph: g.clone(),
            assembler: Arc::clone(&self.assembler),
        };
        self.asked = Some(asked);
        self.assemblies += 1;
        if self.worker.is_none() {
            self.worker = Worker::start();
        }
        let refused = match &self.worker {
            Some(w) => w.jobs.send(job).err().map(|e| e.0),
            None => Some(job),
        };
        // No thread (a browser has none): assembled here.
        if let Some(job) = refused {
            self.worker = None;
            self.take(job.run());
            self.finished.pop();
        }
        self.asked
            .as_ref()
            .map_or(Outcome::Compiling, |a| a.outcome.clone())
    }

    /// Whether the last graph asked for is still with the worker.
    pub fn compiling(&self) -> bool {
        self.asked
            .as_ref()
            .is_some_and(|a| a.outcome == Outcome::Compiling)
    }

    /// Takes what the worker has answered, without waiting.
    pub fn poll(&mut self) {
        while let Some(w) = &self.worker {
            match w.done.try_recv() {
                Ok(done) => self.take(done),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => self.stopped(),
            }
        }
    }

    /// Waits for the last graph asked for: the opening's, which has no last valid code to show, and the tests'.
    pub fn wait(&mut self) {
        while self.compiling() {
            match self.worker.as_ref().map(|w| w.done.recv()) {
                Some(Ok(done)) => self.take(done),
                _ => self.stopped(),
            }
        }
    }

    /// The outcomes the worker has answered since this was last called.
    pub fn finished(&mut self) -> Vec<Outcome> {
        self.poll();
        std::mem::take(&mut self.finished)
    }

    /// Takes the worker's answer to the last graph asked for; one to a graph superseded since is dropped.
    fn take(&mut self, done: Done) {
        let newest = self.asked.as_ref().is_some_and(|a| a.ticket == done.ticket);
        #[cfg(test)]
        let newest = newest || self.completion_order;
        if !newest {
            return;
        }
        let outcome = match done.result {
            Ok((code, ms)) => {
                self.valid = Some((code, ms));
                Outcome::Assembled(ms)
            }
            Err(why) => Outcome::Failed(why),
        };
        self.settle(outcome);
    }

    /// The worker ended without its answer: the graph does not compile, and the next is handed to a new worker.
    fn stopped(&mut self) {
        self.worker = None;
        if self.compiling() {
            self.settle(Outcome::Failed("the assembler stopped".to_owned()));
        }
    }

    /// The last graph asked for came to `outcome`.
    fn settle(&mut self, outcome: Outcome) {
        if let Some(a) = &mut self.asked {
            a.outcome = outcome.clone();
        }
        self.content = None;
        self.finished.push(outcome);
    }

    /// The graph to draw for `g`, the graph as it stands: `g` while its code is what was last compiled, so a param edit
    /// shows at once; else the graph last asked for, until it is recompiled.
    pub fn drawn(&self, g: &StainGraph) -> StainGraph {
        match &self.asked {
            Some(a) if a.key != key(g) => a.graph.clone(),
            _ => g.clone(),
        }
    }

    /// The editor's content for `g`, the graph as it stands, with what drawing it reported, `drawn`: the code of the last
    /// that compiled, `g` compiled first, and waited for, when none has been asked for.
    pub fn content(&mut self, g: &StainGraph, drawn: &Drawn) -> Content {
        if self.asked.is_none() {
            self.compile(g);
            self.wait();
            self.finished.clear();
        }
        self.poll();
        if let Some((of, with, content)) = &self.content {
            if of == g && with == drawn {
                return content.clone();
            }
        }
        let content = self.made(g, drawn);
        self.content = Some((g.clone(), drawn.clone(), content.clone()));
        content
    }

    /// [`Compiler::content`], made.
    fn made(&self, g: &StainGraph, drawn: &Drawn) -> Content {
        let held = catalogue();
        let tables = &held.tables;
        let wanted = key(g);
        let stale = self.asked.as_ref().is_some_and(|a| a.key != wanted);
        let mut problems = held.problems.clone();
        let (described, undescribed) = described(g);
        problems.extend(undescribed);
        let mut notes = drawn.notes.clone();
        // An error is its graph's own: while one compiles, the last one's are not shown as its.
        let asked = self.asked.as_ref().map(|a| a.key);
        let compiling = self.compiling()
            || drawn.compiling
            || (drawn.key.is_some() && asked != Some(drawn.key));
        // The pipelines' errors name the node, so where there are any they stand for the assembler's.
        let failed = match self.asked.as_ref().map(|a| &a.outcome) {
            Some(Outcome::Failed(why)) => vec![why.clone()],
            _ => Vec::new(),
        };
        let failed = if compiling {
            Vec::new()
        } else if drawn.problems.is_empty() {
            failed
        } else {
            drawn.problems.clone()
        };
        let compiled = match self.asked.as_ref().map(|a| &a.outcome) {
            _ if compiling => "compiling · the last that did stays".to_owned(),
            Some(Outcome::Assembled(ms)) if failed.is_empty() => {
                format!("✔ assembled and validated · {ms:.1} ms")
            }
            _ => "✘ does not compile · the last that did stays".to_owned(),
        };
        problems.extend(failed);
        if stale {
            notes.push("note: edited since; Recompile draws it".to_owned());
        }
        if let Ok(form) = g.canonical() {
            notes.push(format!(
                "note: {} of {} nodes live",
                form.nodes.len(),
                g.nodes().count()
            ));
        }
        let (code, pipeline) = self
            .valid
            .as_ref()
            .map(|(a, _)| (a.code.clone(), a.pipeline.clone()))
            .unwrap_or_default();
        Content {
            library: tables.groups.clone(),
            modes: tables.modes,
            loaded: held.loaded(g),
            code,
            pipeline,
            described,
            node_code: node_code(g),
            compiled,
            compiling,
            problems,
            notes,
            choices: tables.choices.clone(),
            graphs: tables.graphs.clone(),
            yours: Default::default(),
        }
    }
}
