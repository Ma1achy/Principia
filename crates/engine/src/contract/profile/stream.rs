//! The streaming writer (telemetry §5, R-341): the header line first, flushed at once, then each frame record as the
//! session produces it, flushed by a [`Flush`] policy, then the summary line, last. It writes the lines [`write`]
//! writes, compact and in the same order, and refuses what [`write`] refuses, so a session it finishes leaves the
//! same file [`write`] would. It keeps no frame record once the record is written.
//!
//! [`write`]: super::write

use std::io::Write;
use std::time::{Duration, Instant};

use super::{
    check_frame, check_header, write_line, At, FrameRecord, HeaderLineOut, SchemaId, Seens,
    SessionHeader, Summary, SummaryLineOut,
};

/// When the streaming writer flushes its frame lines: at the `frames`-th frame written since the last flush, or at
/// the first frame written once `interval` has passed since it, whichever comes first (R-341). The clock is the
/// caller's: each frame is written with the instant it completed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flush {
    /// The frame lines written since the last flush that make a flush due.
    pub frames: u32,
    /// The time since the last flush that makes a flush due.
    pub interval: Duration,
}

impl Flush {
    /// R-341's flush: every 60 frames or 1 s, whichever comes first.
    pub const R341: Flush = Flush {
        frames: 60,
        interval: Duration::from_secs(1),
    };

    /// Whether a flush is due, `frames` frame lines and `elapsed` after the last.
    fn due(&self, frames: u32, elapsed: Duration) -> bool {
        frames >= self.frames || elapsed >= self.interval
    }
}

/// A schema v1 file being written as its session runs (R-341). [`Stream::start`] writes the header line and flushes
/// it, so a session that stops before its first frame flush still leaves a valid trace, a header line alone (R-298);
/// [`Stream::frame`] writes one frame record and flushes when [`Flush`] says it is due; [`Stream::finish`] writes the
/// summary line and flushes. A session that stops before [`Stream::finish`] leaves the incomplete session R-298 and
/// R-299 define, which [`read`](super::read) reports "session incomplete".
///
/// Give it a buffered writer (a `BufWriter` over a `File`, say): it writes each line in pieces, and only its flushes
/// say when the bytes must reach the writer's destination.
pub struct Stream<W: Write> {
    writer: W,
    flush: Flush,
    /// Frame lines written since the last flush.
    unflushed: u32,
    /// When the last flush was made, by the caller's clock.
    flushed_at: Instant,
    /// Frame lines written, to name a refused frame's line: frame `i` is on line `i + 2`.
    written: usize,
}

impl<W: Write> Stream<W> {
    /// Writes the header line to `writer` and flushes it; `now` is the instant of that flush, by the caller's clock. A
    /// header [`write`](super::write) refuses is refused here, and nothing is written.
    pub fn start(
        mut writer: W,
        header: &SessionHeader,
        flush: Flush,
        now: Instant,
    ) -> Result<Self, serde_json::Error> {
        check_header(&At::Line(1), header).map_err(ser_error)?;
        let line = HeaderLineOut {
            schema: SchemaId::V1,
            header,
        };
        write_line(&mut writer, &line)?;
        writer.flush().map_err(serde_json::Error::io)?;
        Ok(Stream {
            writer,
            flush,
            unflushed: 0,
            flushed_at: now,
            written: 0,
        })
    }

    /// Writes `frame` as the next frame line, then flushes if a flush is due by [`Flush`]; `now` is the instant the
    /// frame completed, by the caller's clock. A frame [`write`](super::write) refuses is refused here, and nothing of
    /// it is written; the lines before it stand.
    pub fn frame(&mut self, frame: &FrameRecord, now: Instant) -> Result<(), serde_json::Error> {
        let line = self.written + 2;
        check_frame(&At::Line(line), frame, &mut Seens::default()).map_err(ser_error)?;
        write_line(&mut self.writer, frame)?;
        self.written += 1;
        self.unflushed += 1;
        let elapsed = now.saturating_duration_since(self.flushed_at);
        if self.flush.due(self.unflushed, elapsed) {
            self.writer.flush().map_err(serde_json::Error::io)?;
            self.unflushed = 0;
            self.flushed_at = now;
        }
        Ok(())
    }

    /// Writes the summary line, last, flushes, and gives the writer back (R-286, R-298).
    pub fn finish(
        mut self,
        leak_flags: &Option<Vec<Summary>>,
        hot_paths: &Option<Vec<Summary>>,
    ) -> Result<W, serde_json::Error> {
        let summary = SummaryLineOut {
            leak_flags,
            hot_paths,
        };
        write_line(&mut self.writer, &summary)?;
        self.writer.flush().map_err(serde_json::Error::io)?;
        Ok(self.writer)
    }
}

fn ser_error(message: String) -> serde_json::Error {
    <serde_json::Error as serde::ser::Error>::custom(message)
}
