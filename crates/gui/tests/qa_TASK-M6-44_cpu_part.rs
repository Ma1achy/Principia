//! QA tests for TASK-M6-44 after R-667 (closes RQ-671, amends R-463 K4), written from REQ-GUI-216's statement and
//! verify detail and from R-667's "What it decides", not from the implementation:
//!
//! - **The gate.** Each frame's CPU part, the frame less its wait on the device (egui's passes, the log's filter and
//!   rows, tessellation, and the CPU's work through wgpu), is judged against 16.7 ms; the rule is the slowest frame's:
//!   in every table, at every log size, the frame whose CPU part is largest must be under it.
//! - **The wait is reported.** Each table gives each row's wait, and its slowest whole frame with that frame's wait.
//! - **The control comparison is reported, not a gate.** For the action arm and the control arm, the frames at or over
//!   16.7 ms and at or over 10 ms, and a one-sided Fisher exact p of the action arm having more of them. Neither count
//!   nor p passes or fails anything.
//!
//! The 16.7 ms on the human's Mac is the benchmark's (R-186); these hold the harness's judgement and report to the
//! ruling on synthetic tables of known values, and the split of a real draw on the mock's device. Each check runs its
//! own control (R-176) through [`rejects`].
// The file name `qa_TASK-M6-44_cpu_part` gives a crate name that is not snake case.
#![allow(non_snake_case)]

use gui::bench::{self, Arms, FrameTime, Row, Table, Tail};

/// One frame at 60 fps, in milliseconds (R-463, R-667).
const ONE_FRAME_MS: f64 = 1000.0 / 60.0;

/// Asserts that `check` panics: the check can fail (R-176's control, run in the test).
fn rejects(what: &str, check: impl FnOnce()) {
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check));
    assert!(
        outcome.is_err(),
        "the check passed on {what}, which it must reject"
    );
}

fn table(log: usize, rows: Vec<Row>) -> Table {
    Table { log, rows }
}

fn row(frame: &'static str, times: Vec<FrameTime>) -> Row {
    Row { frame, times }
}

fn cpu(layout_ms: f64) -> FrameTime {
    FrameTime {
        layout_ms,
        passes: 1,
        ..FrameTime::default()
    }
}

fn waited(layout_ms: f64, wait_ms: f64) -> FrameTime {
    FrameTime {
        wait_ms,
        ..cpu(layout_ms)
    }
}

fn judged(t: &Table) -> (bool, bool) {
    (t.meets_target(), bench::render_table(t).contains("MISSED"))
}

// --- The gate: the CPU part, every part of it, and none of the wait --------------------------------------------------

#[test]
fn qa_m6_44_r667_the_wait_is_not_judged() {
    assert!((bench::TARGET_MS - ONE_FRAME_MS).abs() < 1e-12, "16.7 ms");
    // A frame whose CPU part is 1 ms and whose wait on the device is far past a frame: met.
    let t = table(0, vec![row("open: click", vec![waited(1.0, 200.0)])]);
    assert_eq!(judged(&t), (true, false), "{}", bench::render_table(&t));
    // Control: judged on the whole frame, the rule R-667 replaced, the same table misses.
    let whole = t.rows[0].times[0].total_ms();
    rejects("the whole frame judged", || assert!(whole < ONE_FRAME_MS));
}

#[test]
fn qa_m6_44_r667_each_part_of_the_cpu_is_judged() {
    // R-667: egui's passes, the log's filter and rows, tessellation, and the CPU's work through wgpu. Each alone at one
    // frame misses; each just under it, with any wait, is met.
    type Set = fn(&mut FrameTime, f64);
    let parts: [(&str, Set); 5] = [
        ("layout", |f, v| f.layout_ms = v),
        ("filter", |f, v| f.filter_ms = v),
        ("rows", |f, v| f.rows_ms = v),
        ("tessellate", |f, v| f.tessellate_ms = v),
        ("submit", |f, v| f.submit_ms = v),
    ];
    for (name, set) in parts {
        let mut at = FrameTime {
            wait_ms: 0.0,
            ..FrameTime::default()
        };
        set(&mut at, ONE_FRAME_MS);
        assert_eq!(at.cpu_ms(), ONE_FRAME_MS, "{name} in the CPU part");
        let t = table(1_000, vec![row("close: click", vec![at])]);
        assert_eq!(judged(&t), (false, true), "{name} at one frame");
        let mut under = FrameTime {
            wait_ms: 50.0,
            ..FrameTime::default()
        };
        set(&mut under, ONE_FRAME_MS - 0.01);
        let t = table(1_000, vec![row("close: click", vec![under])]);
        assert_eq!(judged(&t), (true, false), "{name} under one frame");
    }
    // The parts add: two halves of a frame miss together, though each alone is met.
    let halves = FrameTime {
        layout_ms: ONE_FRAME_MS / 2.0,
        submit_ms: ONE_FRAME_MS / 2.0,
        ..FrameTime::default()
    };
    let t = table(0, vec![row("open: click", vec![halves])]);
    assert_eq!(judged(&t), (false, true), "the parts' sum");
    // The wait is not in it, and the whole frame is the CPU part and the wait.
    let f = FrameTime {
        layout_ms: 1.0,
        filter_ms: 2.0,
        rows_ms: 3.0,
        tessellate_ms: 4.0,
        submit_ms: 5.0,
        wait_ms: 6.0,
        passes: 1,
    };
    assert!((f.cpu_ms() - 15.0).abs() < 1e-12, "{}", f.cpu_ms());
    assert!((f.total_ms() - 21.0).abs() < 1e-12, "{}", f.total_ms());
    rejects("the wait in the CPU part", || {
        assert!((f.cpu_ms() - 21.0).abs() < 1e-12)
    });
}

#[test]
fn qa_m6_44_r667_the_slowest_cpu_part_of_any_frame_is_judged() {
    // A row's slowest whole frame, by its wait, has a small CPU part; another cycle of the same row has the larger CPU
    // part. The rule is the frame whose CPU part is largest, of every frame measured, not the row's slowest whole frame.
    let t = table(
        CONSOLE_ROWS,
        vec![
            row("closed, before", vec![cpu(2.0), cpu(3.0)]),
            row("open: click", vec![waited(1.0, 100.0), cpu(17.0)]),
        ],
    );
    assert_eq!(judged(&t), (false, true), "{}", bench::render_table(&t));
    assert_eq!(t.worst_cpu(), ("open: click", 17.0));
    // Control: the row's slowest whole frame is the one that waited, which would pass.
    let slowest = t.rows[1].slowest();
    rejects("the row's slowest whole frame judged", || {
        assert!(slowest.cpu_ms() >= ONE_FRAME_MS)
    });
    // Any cycle, any row: the last cycle of the first row.
    let t = table(
        0,
        vec![
            row("closed, before", vec![cpu(1.0), cpu(1.0), cpu(16.7)]),
            row("open: click", vec![cpu(5.0)]),
        ],
    );
    assert_eq!(judged(&t), (false, true), "the last cycle");
}

/// `CONSOLE_ROWS`, R-463 K4's largest log.
const CONSOLE_ROWS: usize = gui::app::CONSOLE_ROWS;

#[test]
fn qa_m6_44_r667_every_size_is_judged_in_the_report() {
    // In every table, at every log size: one size's miss is the report's, whatever the others.
    let arm = |log: usize, worst: f64| Arms {
        action: table(log, vec![row("open: click", vec![cpu(1.0), cpu(worst)])]),
        control: table(log, vec![row("idle 1", vec![cpu(1.0), cpu(1.0)])]),
    };
    let met = bench::report(&[arm(0, 4.0), arm(1_000, 4.0), arm(CONSOLE_ROWS, 4.0)]);
    assert!(!met.contains("MISSED"), "{met}");
    for missing in 0..3 {
        let arms: Vec<Arms> = bench::LOG_SIZES
            .iter()
            .enumerate()
            .map(|(i, &log)| arm(log, if i == missing { 20.0 } else { 4.0 }))
            .collect();
        let text = bench::report(&arms);
        assert_eq!(text.matches("MISSED").count(), 1, "size {missing}: {text}");
    }
}

// --- The wait is reported ---------------------------------------------------------------------------------------------

#[test]
fn qa_m6_44_r667_each_rows_wait_and_the_slowest_whole_frame_are_reported() {
    let t = table(
        1_000,
        vec![
            row("closed, before", vec![waited(1.25, 3.5)]),
            row("open: click", vec![waited(2.0, 41.75)]),
        ],
    );
    let text = bench::render_table(&t);
    // Each row's wait, on its own line.
    let line = |frame: &str| -> String {
        text.lines()
            .find(|l| l.starts_with(frame))
            .unwrap_or_else(|| panic!("no row {frame} in {text}"))
            .to_string()
    };
    assert!(line("closed, before").contains("3.50"), "{text}");
    assert!(line("open: click").contains("41.75"), "{text}");
    // The slowest whole frame, 43.75 ms, and that frame's wait, 41.75 ms; yet the CPU part is met.
    assert!(text.contains("43.75"), "{text}");
    assert!(!text.contains("MISSED"), "{text}");
    assert_eq!(t.slowest_whole().0, "open: click");
    assert_eq!(t.slowest_whole().1.wait_ms, 41.75);
    // Control: a table without the wait shows none of it.
    let none = bench::render_table(&table(
        1_000,
        vec![
            row("closed, before", vec![cpu(1.25)]),
            row("open: click", vec![cpu(2.0)]),
        ],
    ));
    rejects("a wait reported where there was none", || {
        assert!(none.contains("41.75"))
    });
}

// --- The control comparison: counted, Fisher's p, and no gate --------------------------------------------------------

#[test]
fn qa_m6_44_r667_fishers_one_sided_p_matches_the_hypergeometric_tail() {
    // Exact values of the hypergeometric's upper tail, P(X >= a) for X the first group's share of the a + c hits.
    let cases: [((usize, usize, usize, usize), f64); 6] = [
        // Fisher's tea: 3 of 4 against 1 of 4: (C(4,3)C(4,1) + C(4,4)C(4,0)) / C(8,4) = 17/70.
        ((3, 4, 1, 4), 17.0 / 70.0),
        // 3 of 10 against 0 of 10: C(17,7) / C(20,10) = 120/1140.
        ((3, 10, 0, 10), 120.0 / 1140.0),
        // 1 of 10 against 1 of 10: 1 - C(18,10)/C(20,10).
        ((1, 10, 1, 10), 1.0 - 43_758.0 / 184_756.0),
        // None in the action arm: certain.
        ((0, 10, 4, 10), 1.0),
        ((0, 120, 0, 120), 1.0),
        // All the hits in the action arm, 2 of 3 against 0 of 3: C(2,2)C(4,1) / C(6,3) = 4/20.
        ((2, 3, 0, 3), 4.0 / 20.0),
    ];
    for ((a, n1, c, n2), want) in cases {
        let p = bench::fisher_greater(a, n1, c, n2);
        assert!(
            (p - want).abs() < 1e-9,
            "({a},{n1},{c},{n2}): {p}, not {want}"
        );
    }
    // One-sided: more in the action arm lowers p; the same counts the other way round do not give the same p.
    let more = bench::fisher_greater(4, 4, 0, 4);
    let fewer = bench::fisher_greater(0, 4, 4, 4);
    assert!((more - 1.0 / 70.0).abs() < 1e-9, "{more}");
    assert!((fewer - 1.0).abs() < 1e-9, "{fewer}");
    rejects("a two-sided p", || {
        assert!((more - 2.0 / 70.0).abs() < 1e-9)
    });
}

#[test]
fn qa_m6_44_r667_the_arms_count_whole_frames_at_or_over_each_limit() {
    assert_eq!(bench::TAIL_MS, 10.0, "R-667's lesser limit");
    // Whole frames, the wait in them: the misses R-667 compares were in the wait. A frame at a limit counts.
    let action = table(
        0,
        vec![
            row("open: click", vec![cpu(5.0), cpu(10.0), cpu(9.999)]),
            row("close: click", vec![waited(1.0, 30.0), cpu(ONE_FRAME_MS)]),
        ],
    );
    let control = table(
        0,
        vec![
            row("idle 1", vec![cpu(1.0), cpu(1.0), cpu(1.0)]),
            row("idle 2", vec![waited(1.0, 11.0), cpu(1.0)]),
        ],
    );
    assert_eq!(
        action.at_or_over(ONE_FRAME_MS),
        2,
        "the target and the wait"
    );
    assert_eq!(action.at_or_over(10.0), 3);
    assert_eq!(control.at_or_over(ONE_FRAME_MS), 0);
    assert_eq!(control.at_or_over(10.0), 1);
    // Over every size's two arms, against every frame measured.
    let arms = [
        Arms {
            action: action.clone(),
            control: control.clone(),
        },
        Arms { action, control },
    ];
    let tail = Tail::of(&arms, 10.0);
    assert_eq!(tail.action, (6, 10));
    assert_eq!(tail.control, (2, 10));
    assert!((tail.p() - bench::fisher_greater(6, 10, 2, 10)).abs() < 1e-12);
    let tail = Tail::of(&arms, ONE_FRAME_MS);
    assert_eq!((tail.action, tail.control), ((4, 10), (0, 10)));
    rejects("a frame at the limit not counted", || {
        assert_eq!(arms[0].action.at_or_over(10.0), 2)
    });
}

#[test]
fn qa_m6_44_r667_the_comparison_is_reported_and_is_no_gate() {
    // The action arm's tail far past the control's, all of it in the wait: p small, every CPU part met. The report
    // gives the counts and p at both limits, and nothing in it fails.
    let action = |log| {
        table(
            log,
            vec![row(
                "open: click",
                vec![
                    waited(1.0, 40.0),
                    waited(1.0, 40.0),
                    waited(1.0, 40.0),
                    cpu(1.0),
                ],
            )],
        )
    };
    let control = |log| table(log, vec![row("idle 1", vec![cpu(1.0); 4])]);
    let arms: Vec<Arms> = bench::LOG_SIZES
        .iter()
        .map(|&log| Arms {
            action: action(log),
            control: control(log),
        })
        .collect();
    let p = Tail::of(&arms, ONE_FRAME_MS).p();
    assert!(p < 0.001, "{p}");
    let text = bench::report(&arms);
    assert!(!text.contains("MISSED"), "the comparison gated: {text}");
    assert!(text.contains("Fisher"), "{text}");
    assert!(text.contains(&format!("{p:.3}")), "p {p:.3} in {text}");
    for limit in ["16.7", "10.0"] {
        assert!(
            text.lines()
                .any(|l| l.contains("Fisher") && l.contains(limit) && l.contains("9 of 12")),
            "{limit}: {text}"
        );
    }
    // Control: the same report on a CPU part over the target is a miss, so the report can fail.
    let mut over = arms.clone();
    over[1].action.rows[0].times[3] = cpu(18.0);
    rejects("a report with a CPU miss", || {
        assert!(!bench::report(&over).contains("MISSED"))
    });
}

// --- A real draw: its CPU work and its wait apart ---------------------------------------------------------------------

#[test]
fn qa_m6_44_r667_a_draw_on_the_device_splits_its_cpu_from_its_wait() {
    let canvas = gui::cli::mock::canvas_flagging(None).expect("the mock's device");
    let arms = bench::measure_arms(&[0, 1_000], 1, Some(canvas));
    for arm in &arms {
        for t in [&arm.action, &arm.control] {
            for r in &t.rows {
                for f in &r.times {
                    assert!(f.submit_ms > 0.0, "log {} {}: no submit", t.log, r.frame);
                    assert!(f.wait_ms >= 0.0, "log {} {}", t.log, r.frame);
                    assert!(
                        (f.total_ms() - f.cpu_ms() - f.wait_ms).abs() < 1e-9,
                        "log {} {}",
                        t.log,
                        r.frame
                    );
                }
            }
        }
        // The control arm: no console, no input; the action arm draws the console's rows.
        assert!(
            arm.control
                .rows
                .iter()
                .flat_map(|r| &r.times)
                .all(|f| f.filter_ms == 0.0 && f.rows_ms == 0.0),
            "log {}: the control drew the console",
            arm.action.log
        );
        let action_rows: f64 = arm
            .action
            .rows
            .iter()
            .flat_map(|r| &r.times)
            .map(|f| f.rows_ms)
            .sum();
        assert!(
            action_rows > 0.0,
            "log {}: the action drew no console",
            arm.action.log
        );
        rejects("an action arm with no console", || {
            assert!(action_rows == 0.0)
        });
        // The same frames in both arms.
        let count = |t: &Table| t.rows.iter().map(|r| r.times.len()).sum::<usize>();
        assert_eq!(count(&arm.action), count(&arm.control));
    }
    // Control: without a device there is no submit, so the check above can fail.
    let none = bench::measure_arms(&[0], 1, None);
    let submit = none[0].action.rows[0].times[0].submit_ms;
    rejects("a run with no device submitting", || assert!(submit > 0.0));
}
