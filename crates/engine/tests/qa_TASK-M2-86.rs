//! QA's tests for TASK-M2-86, the first slice, written from REQ-DEC-057's statement and verify detail (R-926 items 5
//! and 6; R-935's RQ-766 (a)):
//! - predicate (3), "each narrowed component is the correctly rounded value of its f64 evaluation (within half an f32
//!   ulp of it)": `narrow` against an exact midpoint oracle, round to nearest, ties to even, over every f64 bit
//!   pattern, the f32 subnormals, the overflow edge, signed zeros and NaN (`qa_m2_86_narrow_*`);
//! - predicate (1), "the host-written records of a quad equal `write_ic` of the same narrowed `Ic<f32>`, word for
//!   word", each sample's record written alone into its own buffer, and the table's node the chart's own f64 decode at
//!   the sample's f64 point, not at the point rounded to f32 (`qa_m2_86_records_*`, `qa_m2_86_table_*`);
//! - predicate (2), "two runs of the slice give the same records, bitwise", the second run on another thread after
//!   the other charts' runs (`qa_m2_86_runs_*`);
//! - predicate (4), "equal records integrate to equal `SimState`s, bitwise", records from two separate evaluations,
//!   marched on two threads at once, on two occupants (`qa_m2_86_equal_records_*`);
//! - the grids, "one quad grid of 256 × 256 samples over each chart's home window and the same at zoom 2¹⁰ and 2²⁰
//!   about an O(1) point (each latent coordinate 0.7) and about the chart's origin", read through each chart's own map
//!   (`qa_m2_86_grids_*`).
//!
//! The record and SimState checks run on small quads (REQ-DEC-057's grids at fewer samples) and a short horizon to keep
//! the debug suite within its limits; the predicates are per sample, so the sample count does not change what they
//! assert. Every test has its negative control (R-176).
//!
//! **Domain** (pitfalls §10): each decode at its fixed chart and window; each march from its record alone.

use engine::contract::sim_config::Window;
use engine::first_slice::{evaluate, forward_table, march, narrow, outcome, uniforms, Quad};
use kernel::chart::{Chart, InvariantLE, Latent, ShapeSphere};
use kernel::decode::Ic;
use kernel::generated::links::Defaults;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::stage::decode::{ic_words, read_ic, write_ic};
use proptest::prelude::*;
use validation::first_slice::{Centre, Grid, Preset, GRIDS, POINT, SIDE};
use validation::negative_control;

/// The record checks' quads: REQ-DEC-057's grids at this many samples across and up.
const SMALL: [usize; 2] = [24, 16];

/// The march checks' horizon.
const T: f64 = 2.0;

// ── Predicate 3: round to nearest, ties to even ──────────────────────────────────────────────────────────────────────

/// `2¹²⁸ − 2¹⁰³`, the midpoint between `f32::MAX` and the first power of two past it: from there on a finite f64
/// rounds to infinity (the tie goes to the even significand, infinity's).
fn overflow_edge() -> f64 {
    f64::powi(2.0, 128) - f64::powi(2.0, 103)
}

/// The f32 IEEE 754 round to nearest, ties to even, gives for `x`, by an oracle that never narrows with `as`: `y` is
/// it iff `x` lies between the exact f64 midpoints of `y` and its two f32 neighbours, and on a midpoint `y`'s
/// significand is even.
fn is_nearest_even(x: f64, y: f32) -> Result<(), String> {
    if x.is_nan() {
        return if y.is_nan() {
            Ok(())
        } else {
            Err(format!("NaN narrowed to {y:e}"))
        };
    }
    if x.is_infinite() || x.abs() >= overflow_edge() {
        return if y.is_infinite() && y.is_sign_negative() == x.is_sign_negative() {
            Ok(())
        } else {
            Err(format!("{x:e} past the overflow edge narrowed to {y:e}"))
        };
    }
    if !y.is_finite() {
        return Err(format!("{x:e}, below the overflow edge, narrowed to {y:e}"));
    }
    if x == 0.0 || y == 0.0 {
        // A zero, or a value below half the least subnormal, keeps its sign.
        if y.is_sign_negative() != x.is_sign_negative() {
            return Err(format!("{x:e} narrowed to {y:e}, the sign lost"));
        }
    }
    let wide = f64::from(y);
    // f32 neighbours, their midpoints with y exact in f64 (25 significant bits at most).
    let up = y.next_up();
    let down = y.next_down();
    let hi = if up.is_finite() {
        (wide + f64::from(up)) / 2.0
    } else {
        overflow_edge()
    };
    let lo = if down.is_finite() {
        (wide + f64::from(down)) / 2.0
    } else {
        -overflow_edge()
    };
    let even = y.to_bits() & 1 == 0 || y == 0.0;
    let inside = (lo < x && x < hi) || ((x == lo || x == hi) && even);
    if inside {
        Ok(())
    } else {
        Err(format!(
            "{x:e} narrowed to {y:e}, not its nearest f32 (ties to even): the interval is [{lo:e}, {hi:e}]"
        ))
    }
}

/// The fifteen components of `ic`, in a fixed order.
fn parts<R: Copy>(ic: &Ic<R>) -> [R; 15] {
    [
        ic.m[0], ic.m[1], ic.m[2], ic.r[0][0], ic.r[0][1], ic.r[1][0], ic.r[1][1], ic.r[2][0],
        ic.r[2][1], ic.p[0][0], ic.p[0][1], ic.p[1][0], ic.p[1][1], ic.p[2][0], ic.p[2][1],
    ]
}

/// The `Ic<f64>` with these fifteen components.
fn ic_of(x: [f64; 15]) -> Ic<f64> {
    Ic {
        m: [x[0], x[1], x[2]],
        r: [[x[3], x[4]], [x[5], x[6]], [x[7], x[8]]],
        p: [[x[9], x[10]], [x[11], x[12]], [x[13], x[14]]],
    }
}

/// Every component of `narrowing(ic)` is the nearest f32, ties to even, of `ic`'s.
fn check_narrow(ic: &Ic<f64>, narrowing: fn(&Ic<f64>) -> Ic<f32>) -> Result<(), String> {
    let thin = parts(&narrowing(ic));
    for (k, (x, y)) in parts(ic).into_iter().zip(thin).enumerate() {
        is_nearest_even(x, y).map_err(|e| format!("component {k}: {e}"))?;
    }
    Ok(())
}

/// The hard cases: halfway ties both ways, one f64 ulp either side of a tie, the subnormal range and its rounding to
/// zero, the overflow edge and one ulp below it, signed zeros, infinities and NaN.
fn edge_cases() -> Vec<f64> {
    let p = |e: i32| f64::powi(2.0, e);
    let tie_even = 1.0 + p(-24); // between 1 and 1 + 2⁻²³: to 1, the even one
    let tie_odd = 1.0 + 3.0 * p(-24); // between 1 + 2⁻²³ and 1 + 2⁻²²: to 1 + 2⁻²², the even one
    let least = p(-149); // the least f32 subnormal
    vec![
        tie_even,
        tie_odd,
        -tie_odd,
        tie_even.next_up(),
        tie_even.next_down(),
        tie_odd.next_down(),
        1.0 + p(-24) + p(-40),
        least,
        least / 2.0,
        (least / 2.0).next_up(),
        3.0 * least / 2.0,
        -least / 2.0,
        p(-126) - p(-150),
        overflow_edge(),
        overflow_edge().next_down(),
        -overflow_edge(),
        f64::MAX,
        0.0,
        -0.0,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        0.7,
        -1.0 / 3.0,
        std::f64::consts::PI,
    ]
}

fn check_narrow_edges(narrowing: fn(&Ic<f64>) -> Ic<f32>) {
    for chunk in edge_cases().chunks(15) {
        let mut x = [0.5f64; 15];
        x[..chunk.len()].copy_from_slice(chunk);
        if let Err(e) = check_narrow(&ic_of(x), narrowing) {
            panic!("the narrowing is not round to nearest, ties to even: {e}");
        }
    }
}

#[test]
fn qa_m2_86_narrow_edges() {
    check_narrow_edges(narrow);
}

negative_control!(
    qa_m2_86_narrow_edges,
    "a narrowing that truncates toward zero must fail",
    expected = "the narrowing is not round to nearest",
    check_narrow_edges(truncating)
);

/// A narrowing toward zero, the control's.
#[cfg(feature = "controls")]
fn truncating(ic: &Ic<f64>) -> Ic<f32> {
    let t = |x: f64| {
        let y = x as f32;
        if y.is_finite() && f64::from(y).abs() > x.abs() {
            if y > 0.0 {
                y.next_down()
            } else {
                y.next_up()
            }
        } else {
            y
        }
    };
    Ic {
        m: ic.m.map(t),
        r: ic.r.map(|v| v.map(t)),
        p: ic.p.map(|v| v.map(t)),
    }
}

/// Every f64 bit pattern, and every f64 in the f32 range's binades, near-ties included.
fn any_component() -> impl Strategy<Value = f64> {
    prop_oneof![
        any::<u64>().prop_map(f64::from_bits),
        // An f32 with a few low f64 bits set: on or near a tie, in any binade.
        (any::<u32>(), 0u64..(1 << 29)).prop_map(|(b, low)| {
            let y = f32::from_bits(b);
            if y.is_finite() {
                f64::from_bits(f64::from(y).to_bits() ^ low)
            } else {
                f64::from(y)
            }
        }),
        // Exact ties: the midpoint of an f32 and its neighbour up.
        any::<u32>().prop_map(|b| {
            let y = f32::from_bits(b);
            let up = y.next_up();
            if y.is_finite() && up.is_finite() {
                (f64::from(y) + f64::from(up)) / 2.0
            } else {
                f64::from(y)
            }
        }),
    ]
}

fn check_narrow_property(narrowing: fn(&Ic<f64>) -> Ic<f32>) {
    validation::prop::run(&proptest::array::uniform15(any_component()), |x| {
        check_narrow(&ic_of(x), narrowing).map_err(TestCaseError::fail)
    });
}

#[test]
fn qa_m2_86_narrow_property() {
    check_narrow_property(narrow);
}

negative_control!(
    qa_m2_86_narrow_property,
    "a narrowing that truncates toward zero must fail",
    expected = "property failed",
    check_narrow_property(truncating)
);

// ── Predicate 1: each sample's record is `write_ic` of its narrowed node ─────────────────────────────────────────────

/// `C`'s quad of `grid` at [`SMALL`] samples.
fn small<C: Preset>(grid: Grid) -> Quad<C> {
    C::quad(grid, SMALL)
}

/// `records`, the slice's records of `q`: `ic_words` words a sample, and each sample's words those `write_ic` writes
/// for its narrowed node alone, into a buffer of its own; `read_ic` reads back the narrowed node bit for bit.
fn check_records<C: Preset>(q: &Quad<C>, records: &[u32]) {
    let table = forward_table(q).expect("the physics kernel decodes");
    let n = ic_words();
    assert_eq!(n, 15, "a record is fifteen words, the Ic<f32>'s");
    assert_eq!(records.len(), q.len() * n, "the records' length");
    for (i, node) in table.iter().enumerate() {
        let thin = narrow(node);
        let mut own = vec![0u32; n];
        write_ic(&mut own, 0, &thin);
        assert_eq!(
            &records[i * n..(i + 1) * n],
            &own[..],
            "sample {i}: the record is not write_ic of the narrowed node"
        );
        let back = parts(&read_ic(records, i)).map(f32::to_bits);
        assert_eq!(back, parts(&thin).map(f32::to_bits), "sample {i}: read_ic");
    }
}

fn check_charts_records(perturb: fn(&mut Vec<u32>)) {
    fn one<C: Preset>(perturb: fn(&mut Vec<u32>)) {
        for grid in GRIDS {
            let q = small::<C>(grid);
            let mut r = evaluate(&q).expect("the physics kernel decodes");
            perturb(&mut r);
            check_records(&q, &r);
        }
    }
    one::<Latent<f32>>(perturb);
    one::<ShapeSphere<f32>>(perturb);
    one::<InvariantLE<f32>>(perturb);
}

#[test]
fn qa_m2_86_records_are_write_ic_per_sample() {
    check_charts_records(|_| {});
}

negative_control!(
    qa_m2_86_records_are_write_ic_per_sample,
    "records shifted by one word must fail",
    expected = "is not write_ic of the narrowed node",
    check_charts_records(|r| r.rotate_left(1))
);

/// Each node of `q`'s forward table is the chart's own f64 decode at the sample's f64 point, bit for bit; and,
/// somewhere on the quad, not the decode at that point rounded to f32: the evaluation is in f64 from the point on.
fn check_table<C: Preset>(q: &Quad<C>, table: &[Ic<f64>]) {
    let mut moved = 0;
    for (i, node) in table.iter().enumerate() {
        let [u, v] = q.uv(i);
        let own = q.chart.decode_at::<Defaults, f64>(u, v).ic;
        assert_eq!(
            parts(node).map(f64::to_bits),
            parts(&own).map(f64::to_bits),
            "sample {i}: the table's node is not the chart's f64 decode at its point"
        );
        let rounded = q
            .chart
            .decode_at::<Defaults, f64>(f64::from(u as f32), f64::from(v as f32))
            .ic;
        moved += usize::from(parts(node).map(f64::to_bits) != parts(&rounded).map(f64::to_bits));
    }
    assert!(
        moved > 0,
        "the table is the decode at the f32-rounded points: the evaluation is not f64 from the point on"
    );
}

/// Off-dyadic points: the curved charts zoomed `2²⁰` about the O(1) point, where the f32 rounding of `(u, v)` is
/// larger than the sample spacing.
fn check_tables(table: fn(&Quad<ShapeSphere<f32>>) -> Vec<Ic<f64>>) {
    let s = small::<ShapeSphere<f32>>(GRIDS[2]);
    check_table(&s, &table(&s));
    let e = small::<InvariantLE<f32>>(GRIDS[2]);
    check_table(&e, &forward_table(&e).expect("the physics kernel decodes"));
}

#[test]
fn qa_m2_86_table_is_the_f64_decode_at_the_f64_point() {
    check_tables(|q| forward_table(q).expect("the physics kernel decodes"));
}

negative_control!(
    qa_m2_86_table_is_the_f64_decode_at_the_f64_point,
    "a table evaluated at the f32-rounded points must fail",
    expected = "the table's node is not the chart's f64 decode at its point",
    check_tables(|q| (0..q.len())
        .map(|i| {
            use kernel::chart::ChartDecode;
            let [u, v] = q.uv(i);
            q.chart
                .decode_at::<Defaults, f64>(f64::from(u as f32), f64::from(v as f32))
                .ic
        })
        .collect())
);

// ── Predicate 2: two runs, the same records, whatever ran between ────────────────────────────────────────────────────

/// Every chart's records on every grid, in the order latent, sphere, `(L_z, E)`, or the reverse.
fn all_records(reverse: bool) -> Vec<Vec<u32>> {
    fn of<C: Preset>() -> Vec<Vec<u32>> {
        GRIDS
            .iter()
            .map(|&g| evaluate(&small::<C>(g)).expect("the physics kernel decodes"))
            .collect()
    }
    if reverse {
        let (e, s, l) = (
            of::<InvariantLE<f32>>(),
            of::<ShapeSphere<f32>>(),
            of::<Latent<f32>>(),
        );
        [l, s, e].concat()
    } else {
        [
            of::<Latent<f32>>(),
            of::<ShapeSphere<f32>>(),
            of::<InvariantLE<f32>>(),
        ]
        .concat()
    }
}

fn check_two_runs(second: fn() -> Vec<Vec<u32>>) {
    let first = all_records(false);
    let other = std::thread::spawn(second).join().expect("the second run");
    assert_eq!(first.len(), other.len(), "the runs' quads");
    for (k, (a, b)) in first.iter().zip(&other).enumerate() {
        assert!(
            a == b,
            "quad {k}: two runs of the slice gave different records"
        );
    }
}

#[test]
fn qa_m2_86_runs_bitwise_across_threads_and_orders() {
    check_two_runs(|| all_records(true));
}

negative_control!(
    qa_m2_86_runs_bitwise_across_threads_and_orders,
    "today's f32 decode as the second run must fail",
    expected = "two runs of the slice gave different records",
    check_two_runs(|| {
        fn of<C: Preset>() -> Vec<Vec<u32>> {
            GRIDS
                .iter()
                .map(|&g| {
                    let q = small::<C>(g);
                    let old = engine::first_slice::today(&q);
                    let mut w = vec![0u32; q.len() * ic_words()];
                    for (i, d) in old.iter().enumerate() {
                        write_ic(&mut w, i, &d.ic);
                    }
                    w
                })
                .collect()
        }
        [
            of::<Latent<f32>>(),
            of::<ShapeSphere<f32>>(),
            of::<InvariantLE<f32>>(),
        ]
        .concat()
    })
);

// ── Predicate 4: equal records, equal `SimState`s ────────────────────────────────────────────────────────────────────

/// A `SimState` as text: `Debug` prints each float's shortest round-trip form and each integer, so equal text is equal
/// bits outside NaN payloads.
fn state_text<A: kernel::driver::advance::Advance>(ic: &Ic<f32>, t: f64) -> String {
    let u = uniforms(t).expect("a valid horizon");
    format!("{:?}", march::<A>(ic, &u))
}

/// The records of each chart's home quad, at 4 × 2 samples, from two separate evaluations: where equal, each marched
/// on two threads at once on occupant `A` gives the same `SimState`; `nudge` changes the second's record first.
fn check_equal_march<A: kernel::driver::advance::Advance>(nudge: fn(Ic<f32>) -> Ic<f32>) {
    fn records<C: Preset + Send + Sync>() -> (Vec<u32>, Vec<u32>) {
        let q = C::quad(GRIDS[0], [4, 2]);
        let a = evaluate(&q).expect("the physics kernel decodes");
        let b = std::thread::scope(|s| {
            s.spawn(|| evaluate(&q).expect("the physics kernel decodes"))
                .join()
                .expect("the second evaluation")
        });
        (a, b)
    }
    let mut marched = 0;
    for (a, b) in [
        records::<Latent<f32>>(),
        records::<ShapeSphere<f32>>(),
        records::<InvariantLE<f32>>(),
    ] {
        for i in 0..a.len() / ic_words() {
            let (ra, rb) = (read_ic(&a, i), read_ic(&b, i));
            assert_eq!(
                parts(&ra).map(f32::to_bits),
                parts(&rb).map(f32::to_bits),
                "sample {i}: two evaluations gave different records"
            );
            let rb = nudge(rb);
            let (sa, sb) = std::thread::scope(|s| {
                let ha = s.spawn(|| state_text::<A>(&ra, T));
                let hb = s.spawn(|| state_text::<A>(&rb, T));
                (ha.join().expect("march a"), hb.join().expect("march b"))
            });
            assert!(
                sa == sb,
                "sample {i}: equal records integrated to different SimStates:\n{sa}\n{sb}"
            );
            let u = uniforms(T).expect("a valid horizon");
            assert_eq!(
                outcome::<A>(&ra, &u),
                outcome::<A>(&rb, &u),
                "sample {i}: the outcomes"
            );
            marched += 1;
        }
    }
    assert_eq!(marched, 24, "the records marched");
}

#[test]
fn qa_m2_86_equal_records_integrate_equal_kdk() {
    check_equal_march::<Kdk>(|ic| ic);
}

#[test]
fn qa_m2_86_equal_records_integrate_equal_yoshida4() {
    check_equal_march::<Yoshida4>(|ic| ic);
}

/// `ic` with its `r₀ₓ` one f32 up.
#[cfg(feature = "controls")]
fn nudged(mut ic: Ic<f32>) -> Ic<f32> {
    ic.r[0][0] = ic.r[0][0].next_up();
    ic
}

negative_control!(
    qa_m2_86_equal_records_integrate_equal_kdk,
    "a record one float off in one component must integrate differently",
    expected = "equal records integrated to different SimStates",
    check_equal_march::<Kdk>(nudged)
);

negative_control!(
    qa_m2_86_equal_records_integrate_equal_yoshida4,
    "a record one float off in one component must integrate differently",
    expected = "equal records integrated to different SimStates",
    check_equal_march::<Yoshida4>(nudged)
);

// ── The grids: 256 × 256, home, 2¹⁰ and 2²⁰ about the O(1) point and about the origin ────────────────────────────────

/// REQ-DEC-057's five grids, each once.
fn check_grid_list(grids: &[Grid]) {
    assert_eq!(SIDE, 256, "a quad's side");
    let mut want = vec![
        (0, Centre::Home),
        (10, Centre::Point),
        (20, Centre::Point),
        (10, Centre::Origin),
        (20, Centre::Origin),
    ];
    let mut have: Vec<(u32, Centre)> = grids.iter().map(|g| (g.zoom, g.centre)).collect();
    let key = |c: &(u32, Centre)| (c.0, format!("{:?}", c.1));
    want.sort_by_key(key);
    have.sort_by_key(key);
    assert_eq!(have, want, "REQ-DEC-057's grids");
    assert_eq!(POINT, 0.7, "the O(1) point's coordinate");
}

#[test]
fn qa_m2_86_grids_are_req_dec_057s() {
    check_grid_list(&GRIDS);
}

negative_control!(
    qa_m2_86_grids_are_req_dec_057s,
    "a grid list missing the 2²⁰ zoom about the origin must fail",
    expected = "REQ-DEC-057's grids",
    check_grid_list(&GRIDS[..4])
);

/// The latent chart's quads through its own map: the quad's middle is `z = 0.7` (rounded once to f32) in every
/// coordinate about the point, `0` about the origin, the preset's `z₀` at home; the plane's directions home's times
/// `2⁻ᵏ`, exactly, and every quad is 256 × 256.
fn check_latent(quad: fn(Grid) -> Quad<Latent<f32>>) {
    let home = quad(GRIDS[0]);
    for g in GRIDS {
        let q = quad(g);
        assert_eq!(q.size, [SIDE, SIDE], "{}: the quad's samples", g.name());
        assert_eq!(
            q.window.at(0.5, 0.5),
            [0.5, 0.5],
            "{}: the latent window's middle",
            g.name()
        );
        let mid = latent_z(&q.chart, 0.5, 0.5);
        let want: [f64; 8] = match g.centre {
            Centre::Home => home.chart.z0.map(f64::from),
            Centre::Point => [f64::from(POINT as f32); 8],
            Centre::Origin => [0.0; 8],
        };
        assert_eq!(mid, want, "{}: the latent quad's middle", g.name());
        // R-83: the zoom is one common scale on q₁ and q₂, exact at f32 for a power of two.
        let k = f64::powi(2.0, -(g.zoom as i32)) as f32;
        assert_eq!(
            q.chart.q1,
            home.chart.q1.map(|x| x * k),
            "{}: q1 not home's times 2^-k",
            g.name()
        );
        assert_eq!(
            q.chart.q2,
            home.chart.q2.map(|x| x * k),
            "{}: q2 not home's times 2^-k",
            g.name()
        );
    }
}

/// The latent chart's `z` at `(u, v)`, at f64: its eight coordinates.
fn latent_z(chart: &Latent<f32>, u: f64, v: f64) -> [f64; 8] {
    chart.map::<f64>(u, v).z.z
}

#[test]
fn qa_m2_86_grids_latent() {
    check_latent(|g| Latent::quad(g, [SIDE, SIDE]));
}

negative_control!(
    qa_m2_86_grids_latent,
    "a latent quad zoomed about the plane's corner must fail",
    expected = "the latent quad's middle",
    check_latent(|g| {
        let mut q = Latent::quad(g, [SIDE, SIDE]);
        q.chart.z0 = std::array::from_fn(|k| q.chart.z0[k] - q.chart.q1[k] - q.chart.q2[k]);
        q
    })
);

/// A curved chart's window: its middle at `centre`, its half-width home's times `2⁻ᵏ`, 256 × 256 samples.
fn check_window<C: Preset>(q: &Quad<C>, g: Grid, centre: [f64; 2]) {
    let home = Window::SQUARE;
    let k = f64::powi(2.0, -(g.zoom as i32));
    assert_eq!(q.size, [SIDE, SIDE], "{}: the quad's samples", g.name());
    assert_eq!(
        q.window.at(0.5, 0.5),
        centre,
        "{}: the window's middle",
        g.name()
    );
    assert_eq!(
        q.window.half,
        home.half.map(|h| h * k),
        "{}: the window's half-width",
        g.name()
    );
}

/// The shape sphere's quads: about the O(1) point `(0.7, 0.7)`, and about its origin, where the exponential map's
/// offset `d` is zero, so the map gives its centre `n₀` exactly.
fn check_sphere(quad: fn(Grid) -> Quad<ShapeSphere<f32>>) {
    for g in GRIDS {
        let q = quad(g);
        let mid = q.window.at(0.5, 0.5);
        match g.centre {
            Centre::Home => check_window(&q, g, Window::SQUARE.centre),
            Centre::Point => check_window(&q, g, [POINT, POINT]),
            Centre::Origin => {
                check_window(&q, g, mid);
                assert_eq!(
                    q.chart.point::<f64>(mid[0], mid[1]),
                    q.chart.n0.map(f64::from),
                    "{}: the sphere's origin is not the map's centre n0",
                    g.name()
                );
            }
        }
    }
}

#[test]
fn qa_m2_86_grids_shape_sphere() {
    check_sphere(|g| ShapeSphere::quad(g, [SIDE, SIDE]));
}

negative_control!(
    qa_m2_86_grids_shape_sphere,
    "a sphere quad about a point off the map's centre must fail",
    expected = "the sphere's origin is not the map's centre n0",
    check_sphere(|g| {
        let mut q = ShapeSphere::quad(g, [SIDE, SIDE]);
        if g.centre == Centre::Origin {
            q.window.centre[0] += 0.125;
        }
        q
    })
);

/// The `(L_z, E)` chart's quads: about the O(1) point `(0.7, 0.7)`, and about its origin, where the warp's targets are
/// zero, `L_z = 0` and `K* = 0`.
fn check_invariant(quad: fn(Grid) -> Quad<InvariantLE<f32>>) {
    for g in GRIDS {
        let q = quad(g);
        let mid = q.window.at(0.5, 0.5);
        match g.centre {
            Centre::Home => check_window(&q, g, Window::SQUARE.centre),
            Centre::Point => check_window(&q, g, [POINT, POINT]),
            Centre::Origin => {
                check_window(&q, g, mid);
                let t = q.chart.0.targets::<f64>(mid[0], mid[1]);
                assert!(
                    t.lz == 0.0 && t.k == 0.0,
                    "{}: the (L_z, E) chart's origin is not L_z = 0, K = 0: L_z {:e}, K {:e}",
                    g.name(),
                    t.lz,
                    t.k
                );
            }
        }
    }
}

#[test]
fn qa_m2_86_grids_invariant_lz_e() {
    check_invariant(|g| InvariantLE::quad(g, [SIDE, SIDE]));
}

negative_control!(
    qa_m2_86_grids_invariant_lz_e,
    "an (L_z, E) quad about the square's middle must fail",
    expected = "the (L_z, E) chart's origin is not L_z = 0, K = 0",
    check_invariant(|g| {
        let mut q = InvariantLE::quad(g, [SIDE, SIDE]);
        if g.centre == Centre::Origin {
            q.window.centre = [0.5, 0.5];
        }
        q
    })
);
