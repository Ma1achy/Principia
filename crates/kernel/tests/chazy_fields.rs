//! The Chazy fields (chazy drill-down §5.2–§5.6, §7, §12.2 items 2 and 3), on CPU-f64: the folds
//! `chazy_final`, `chazy_status` and `chazy_structure`, the scalar `chazy_q`, the per-body `chazy_escaper` and
//! `chazy_qk`, and the escape freeze.
//!
//! - `chazy_fold_final_*`, `chazy_fold_status_*`: the fold tables, exhaustive over the label enum (a label added breaks
//!   the build: [`chazy_fold_final_is_exhaustive`] compiles the folds' source against an eleventh label), and
//!   collision-terminated samples (pair, triple, `t = 0`) to collision.
//! - `chazy_structure_*`, `chazy_q_*`: the rows on §12.1's fixtures and on each precedence case §5.5 defines, and
//!   each row's `q` against its hand-computed `K/U`.
//! - `chazy_escaper_*`, `chazy_qk_*`: §5.6's per-body fields, against values computed here from
//!   `validation::chazy::State`'s own two-body energies, never the kernel's `out_kup` or `most_separated`.
//! - `chazy_escape_freeze_*`: an escape-terminated sample's fields read its fire-time state.
//!
//! **The controls.** Each check takes the field under test as a function; its control passes one with the property it
//! guards broken, and must fail.

use std::path::PathBuf;
use std::process::Command;

use kernel::chazy::{
    chazy_escaper, chazy_final, chazy_q, chazy_qk, chazy_status, chazy_structure, classify,
    fold_final, fold_status, fold_structure, inertia, most_separated, others, out_kup, pair_kup,
    q_of, ChazyCfg, Final, Motion, Playhead, Previous, Status, Structure,
};
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::nsub_table::{thresholds, NsubConfig};
use kernel::driver::wrapper::{integrate_detecting_rule, integrate_detecting_to, WrapperUniforms};
use kernel::occupant::rk4::Rk4;
use kernel::payload::{
    sd_I_max, sd_detail, sd_state, tm_t_end_step, SimStateBaseOf, STATE_BOUNDED, STATE_COLLISION,
    STATE_ESCAPE, STATE_RUNNING,
};
use validation::chazy::{
    binary_plus_escaper, equilateral_state, ma_hierarchy, two_stage, State, THIRDS,
};
use validation::negative_control;

// ── Fixtures ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// The playhead of fixture `s`: `E` its energy, `R_0 = 1` and a fresh latch, `I_max = I` (§12.1).
fn at(s: &State) -> Playhead<f64> {
    Playhead {
        m: s.m,
        r: s.r,
        v: s.v,
        e: s.energy(),
        r_0: 1.0,
        i_max: inertia(&s.m, &s.r),
    }
}

fn cfg() -> ChazyCfg<f64> {
    ChazyCfg::defaults()
}

/// `s` with every velocity reversed: the same energies, every radial velocity's sign flipped.
fn reversed(s: &State) -> State {
    State {
        v: s.v.map(|q| [-q[0], -q[1]]),
        ..*s
    }
}

/// `s` with its bodies relabelled: new body `k` is old body `perm[k]`.
fn relabelled(s: &State, perm: [usize; 3]) -> State {
    State {
        m: perm.map(|k| s.m[k]),
        r: perm.map(|k| s.r[k]),
        v: perm.map(|k| s.v[k]),
    }
}

/// §5.5's transient overlap state, the review's (written here): equal masses, bodies 0 and 1 at `(0, ∓0.5)` separating
/// along y with pair `K/U = 6.75` (unbound), body 2 at `(10, 0)`, `ρ = 10`, receding radially with outer `K/U = 0.5`.
/// `E ≈ +0.628`; the hierarchy step's gate passes but its pair is unbound and its outer body bound, so `classify`
/// falls through to transient.
fn transient_overlap() -> State {
    let m = THIRDS;
    let mu = m[0] * m[1] / (m[0] + m[1]);
    // K/U = 6.75 at d = 1: ½ μ v_rel² = 6.75 m_0 m_1.
    let v_rel = (2.0 * 6.75 * m[0] * m[1] / mu).sqrt();
    let mb = m[0] + m[1];
    let mu_out = m[2] * mb / (m[2] + mb);
    let v_out = (2.0 * 0.5 * (m[2] * mb / 10.0) / mu_out).sqrt();
    State::com(
        m,
        [[0.0, -0.5], [0.0, 0.5], [10.0, 0.0]],
        [[0.0, -0.5 * v_rel], [0.0, 0.5 * v_rel], [v_out, 0.0]],
    )
}

/// A two-stage H whose escaper co-moves with one body of the unbound pair (written here): bodies 0 and 1 at `(∓0.5, 0)`
/// separating along x with pair `K/U = 2`, body 2 at `(50, 0)` moving with body 1's velocity. Pair `(1, 2)` has `K = 0`,
/// bound; the inner pair and the outer body, the two the rule banded, are unbound.
fn two_stage_comoving() -> State {
    let m = THIRDS;
    // K/U = 2 at d = 1: ½ (1/6) (2w)² = 2/9, w² = 2/3.
    let w = (2.0f64 / 3.0).sqrt();
    State::com(
        m,
        [[-0.5, 0.0], [0.5, 0.0], [50.0, 0.0]],
        [[-w, 0.0], [w, 0.0], [w, 0.0]],
    )
}

// ── The folds (§5.3, §5.4) ────────────────────────────────────────────────────────────

/// §5.3's table, written here from the drill-down.
const FINAL_TABLE: [(Motion, Final); 10] = [
    (Motion::H, Final::H),
    (Motion::Hp, Final::Hp),
    (Motion::P, Final::P),
    (Motion::He, Final::He),
    (Motion::Pe, Final::Pe),
    (Motion::BStable, Final::B),
    (Motion::BSoFar, Final::B),
    (Motion::Excursion, Final::B),
    (Motion::Transient, Final::B),
    (Motion::OsCandidate, Final::Os),
];

/// §5.4's statuses, written here from the drill-down.
const STATUS_TABLE: [(Motion, Status); 10] = [
    (Motion::H, Status::Committed),
    (Motion::Hp, Status::Committed),
    (Motion::P, Status::Committed),
    (Motion::He, Status::Committed),
    (Motion::Pe, Status::Committed),
    (Motion::BStable, Status::BStable),
    (Motion::BSoFar, Status::BSoFar),
    (Motion::Excursion, Status::Excursion),
    (Motion::Transient, Status::Transient),
    (Motion::OsCandidate, Status::OsCandidate),
];

fn check_final_table(fold: fn(Motion) -> Final) {
    assert_eq!(FINAL_TABLE.len(), Motion::ALL.len());
    for (label, want) in FINAL_TABLE {
        assert_eq!(fold(label), want, "chazy_final folds {label:?} wrongly");
    }
    assert_eq!(
        fold(Motion::Transient),
        Final::B,
        "chazy_final folds transient wrongly"
    );
}

#[test]
fn chazy_fold_final_every_label_to_its_class() {
    check_final_table(fold_final);
}

negative_control!(
    chazy_fold_final_every_label_to_its_class,
    "a fold that keeps transient apart from B (to OS) fails the table",
    expected = "chazy_final folds Transient wrongly",
    check_final_table(|l| match l {
        Motion::Transient => Final::Os,
        other => fold_final(other),
    })
);

fn check_status_table(fold: fn(Motion) -> Status) {
    assert_eq!(STATUS_TABLE.len(), Motion::ALL.len());
    for (label, want) in STATUS_TABLE {
        assert_eq!(fold(label), want, "chazy_status maps {label:?} wrongly");
    }
}

#[test]
fn chazy_fold_status_every_label_to_its_status() {
    check_status_table(fold_status);
}

negative_control!(
    chazy_fold_status_every_label_to_its_status,
    "a status that calls B-stable committed (prin-rs's `committed`) fails §5.4's own colour",
    expected = "chazy_status maps BStable wrongly",
    check_status_table(|l| match l {
        Motion::BStable => Status::Committed,
        other => fold_status(other),
    })
);

/// §5.5's rows by label, as the precedence there now writes them.
const STRUCTURE_TABLE: [(Motion, Structure); 10] = [
    (Motion::H, Structure::Dispersal),
    (Motion::Hp, Structure::Dispersal),
    (Motion::P, Structure::Dispersal),
    (Motion::He, Structure::Hierarchy),
    (Motion::Pe, Structure::Hierarchy),
    (Motion::BStable, Structure::Hierarchy),
    (Motion::BSoFar, Structure::Interacting),
    (Motion::Excursion, Structure::Hierarchy),
    (Motion::Transient, Structure::Interacting),
    (Motion::OsCandidate, Structure::Oscillatory),
];

fn check_structure_table(fold: fn(Motion) -> Structure) {
    assert_eq!(STRUCTURE_TABLE.len(), Motion::ALL.len());
    for (label, want) in STRUCTURE_TABLE {
        assert_eq!(
            fold(label),
            want,
            "chazy_structure gives {label:?} the wrong row"
        );
    }
}

#[test]
fn chazy_structure_every_label_to_its_row() {
    check_structure_table(fold_structure);
}

negative_control!(
    chazy_structure_every_label_to_its_row,
    "a fold that reads HP and P as interacting (their geometry unresolved) fails dispersal",
    expected = "chazy_structure gives Hp the wrong row",
    check_structure_table(|l| match l {
        Motion::Hp | Motion::P => Structure::Interacting,
        other => fold_structure(other),
    })
);

/// The run of `ic` by RK4 at `dt = 0.001` for `steps` macro-steps, at collision radius `r_coll`.
fn run(ic: &Ic<f64>, steps: u32, r_coll: f64) -> SimStateBaseOf<f64> {
    integrate_detecting_rule::<Rk4, f64, kernel::detect::escape_rule::DefaultRule<f64>>(
        ic,
        &key(0.001, steps),
        CollisionRadius::from_r_coll(r_coll),
        false,
    )
}

fn key(dt: f64, steps: u32) -> WrapperUniforms<f64> {
    let config = NsubConfig::DEFAULT;
    WrapperUniforms {
        dt_macro: dt,
        horizon_steps: steps,
        n_max: config.n_max,
        thresholds: thresholds(&config).expect("the default substep law generates"),
    }
}

fn ic(s: &State) -> Ic<f64> {
    Ic {
        m: s.m,
        r: s.r,
        p: s.p(),
    }
}

/// Collision-terminated samples, each with the label its stored state would read: a pair collision (a head-on pair
/// falling in, the third body far), a collision at `t = 0` (an IC already inside `r_coll`), and a stored triple
/// collision (detail 3 on the pair collision's stored state: the field reads `state`, never `detail`, §7).
fn collision_samples() -> Vec<(&'static str, SimStateBaseOf<f64>)> {
    let pair = State::com(
        THIRDS,
        [[-0.05, 0.0], [0.05, 0.0], [20.0, 0.0]],
        [[0.0; 2]; 3],
    );
    let ran = run(&ic(&pair), 20_000, 1e-3);
    assert_eq!(
        sd_state(ran.packed_a),
        STATE_COLLISION,
        "the pair fixture does not collide"
    );
    assert!(
        tm_t_end_step(ran.times) > 0,
        "the pair fixture collides at t = 0"
    );
    let inside = State::com(
        THIRDS,
        [[-0.0001, 0.0], [0.0001, 0.0], [1.0, 0.0]],
        [[0.0; 2]; 3],
    );
    let at_zero = run(&ic(&inside), 100, 1e-3);
    assert_eq!(
        sd_state(at_zero.packed_a),
        STATE_COLLISION,
        "the t = 0 fixture does not collide"
    );
    assert_eq!(
        tm_t_end_step(at_zero.times),
        0,
        "the t = 0 fixture does not collide at t = 0"
    );
    let mut triple = ran;
    triple.packed_a = kernel::payload::set_detail(triple.packed_a, 3);
    assert_eq!(sd_detail(triple.packed_a), 3);
    vec![("pair", ran), ("t = 0", at_zero), ("triple", triple)]
}

/// The stored sample's fields through the read side's inputs: `R_0` from the IC (`R_0 = √(I_0/M)`).
fn read_final(state: u32, s: &SimStateBaseOf<f64>, m: [f64; 3], r_0: f64) -> (u32, u32) {
    let code = sd_I_max(s.packed_a);
    (
        chazy_final(state, m, s.r, s.p, s.E_0, r_0, code),
        chazy_status(state, m, s.r, s.p, s.E_0, r_0, code),
    )
}

type ReadFinal = fn(u32, &SimStateBaseOf<f64>, [f64; 3], f64) -> (u32, u32);

fn check_collision(read: ReadFinal) {
    for (name, s) in collision_samples() {
        let (f, st) = read(sd_state(s.packed_a), &s, THIRDS, 1.0);
        assert_eq!(
            f,
            Final::Collision.code(),
            "the {name} collision's chazy_final is not collision"
        );
        assert_eq!(
            st,
            Status::Collision.code(),
            "the {name} collision's chazy_status is not collision"
        );
    }
    // A sample that did not collide reads its motion class, never collision.
    let he = binary_plus_escaper(1.0, 0.0, 50.0, 2.0);
    let s = SimStateBaseOf {
        r: he.r,
        p: he.p(),
        E_0: he.energy(),
        ..collision_samples()[0].1
    };
    for state in [STATE_BOUNDED, STATE_RUNNING, STATE_ESCAPE] {
        let (f, st) = read(state, &s, he.m, 1.0);
        assert_eq!(
            f,
            Final::He.code(),
            "a {state} sample's chazy_final is not its class"
        );
        assert_eq!(
            st,
            Status::Committed.code(),
            "a {state} sample's chazy_status is not its class"
        );
    }
}

#[test]
fn chazy_fold_final_a_collision_terminated_sample_is_collision() {
    check_collision(read_final);
}

negative_control!(
    chazy_fold_final_a_collision_terminated_sample_is_collision,
    "a fold that reads the stored state's motion and not its collision label fails",
    expected = "collision's chazy_final is not collision",
    check_collision(|_, s, m, r_0| read_final(STATE_BOUNDED, s, m, r_0))
);

/// The folds' source, between `kernel::chazy`'s markers.
fn folds_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/chazy.rs");
    let text = std::fs::read_to_string(&path).expect("kernel::chazy's source reads");
    let begin = text
        .find("// The folds: begin.")
        .expect("the folds' begin marker");
    let end = text
        .find("// The folds: end.")
        .expect("the folds' end marker");
    text[begin..end].to_owned()
}

/// Compiles the folds' source against the label enum, with an eleventh label where `extra`; panics where it does not
/// build. The enums are written here, as `kernel::chazy` declares them.
fn check_folds_build(extra: bool) {
    let labels = "H, Hp, P, He, Pe, BStable, BSoFar, Transient, OsCandidate, Excursion";
    let source = format!(
        "#![allow(dead_code)]\n\
         #[derive(Clone, Copy)] pub enum Motion {{ {labels}{} }}\n\
         pub enum Final {{ B, H, Hp, P, He, Pe, Os, Collision }}\n\
         pub enum Status {{ Committed, BStable, BSoFar, Excursion, Transient, OsCandidate, Collision }}\n\
         pub enum Structure {{ Dispersal, Hierarchy, Interacting, Oscillatory }}\n{}",
        if extra { ", Unmapped" } else { "" },
        folds_source()
    );
    let dir = std::env::temp_dir().join(format!(
        "chazy_folds_{}_{}",
        std::process::id(),
        u8::from(extra)
    ));
    std::fs::create_dir_all(&dir).expect("the scratch directory");
    let file = dir.join("folds.rs");
    std::fs::write(&file, source).expect("the folds' scratch source");
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let out = Command::new(rustc)
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--emit",
            "metadata",
            "-o",
        ])
        .arg(dir.join("folds.rmeta"))
        .arg(&file)
        .output()
        .expect("rustc runs");
    let _ = std::fs::remove_dir_all(&dir);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        assert!(
            stderr.contains("E0004"),
            "the folds failed to build for another reason than a non-exhaustive match: {stderr}"
        );
        panic!("the folds do not build: a label is unmapped (E0004)");
    }
}

#[test]
fn chazy_fold_final_is_exhaustive() {
    check_folds_build(false);
}

negative_control!(
    chazy_fold_final_is_exhaustive,
    "an unmapped label fails to build: the folds compiled against an eleventh label",
    expected = "the folds do not build",
    check_folds_build(true)
);

// ── The structure rows and their q (§5.5) ──────────────────────────────────

/// A structure under test: the row and its `q` at a playhead.
type Rows<'a> = &'a dyn Fn(&Playhead<f64>) -> (Structure, f64);

/// The kernel's rows: [`fold_structure`] of the label and [`q_of`].
fn rows(ph: &Playhead<f64>) -> (Structure, f64) {
    let c = cfg();
    let row = fold_structure(classify(ph, &c, &Previous::none()));
    (row, q_of(ph, row, c.eps))
}

/// §5.5's table taken literally (the review's reading): the hierarchy row wherever the most separated body's ratio is
/// above `h_far`, its `q` the outer `K/U`; otherwise the kernel's.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn literal(ph: &Playhead<f64>) -> (Structure, f64) {
    let b = most_separated(ph);
    let (i, j) = others(b);
    let o = out_kup(ph, b, i, j);
    if o.d / pair_kup(ph, i, j).d > cfg().h_far {
        (Structure::Hierarchy, o.k / o.u)
    } else {
        rows(ph)
    }
}

/// The dispersal row's `q` as "the smallest pair `K/U`" alone, with no exception for a bound pair.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn smallest_pair(ph: &Playhead<f64>) -> (Structure, f64) {
    let (row, q) = rows(ph);
    if row == Structure::Dispersal {
        let r = |i, j| {
            let x = pair_kup(ph, i, j);
            x.k / x.u
        };
        (row, r(0, 1).min(r(0, 2)).min(r(1, 2)))
    } else {
        (row, q)
    }
}

/// The interacting and oscillatory rows' `q` from the live kinetic energy, not `E_0`.
#[cfg_attr(not(feature = "controls"), allow(dead_code))]
fn live_kinetic(ph: &Playhead<f64>) -> (Structure, f64) {
    let (row, q) = rows(ph);
    if matches!(row, Structure::Interacting | Structure::Oscillatory) {
        let k: f64 = (0..3)
            .map(|n| 0.5 * ph.m[n] * (ph.v[n][0].powi(2) + ph.v[n][1].powi(2)))
            .sum();
        let u = pair_kup(ph, 0, 1).u + pair_kup(ph, 0, 2).u + pair_kup(ph, 1, 2).u;
        (row, k / u)
    } else {
        (row, q)
    }
}

/// Pass criterion, written before the run: a `q` computed here by hand equals the field's to 64 ulps of f64 relative.
/// The construction (`State::com`'s shift and the escaper's speed) and the field's read (`out_kup`'s centre, `Δv`, `μ`,
/// `K`, `U`) take about thirty roundings between them, each within half an ulp, with no cancellation worse than a
/// factor of two (the centre-of-mass shift subtracts a velocity a third of the escaper's).
const Q_REL: f64 = 64.0 * f64::EPSILON;

fn close(q: f64, want: f64, what: &str) {
    assert!(
        (q - want).abs() <= Q_REL * want.abs().max(f64::MIN_POSITIVE),
        "{what}: q = {q:.17e}, hand-computed {want:.17e}"
    );
}

/// The parabolic band's half-width, `ε`.
fn eps() -> f64 {
    cfg().eps
}

/// Every row and `q` on §12.1's fixtures and §5.5's precedence cases.
fn check_rows(f: Rows) {
    let row = |s: &State| f(&at(s));
    // §12.1: H, HE, PE, excursion, transient, B-stable, B-so-far.
    let apart = equilateral_state(50.0, 2.0);
    assert_eq!(rows(&at(&apart)).0, Structure::Dispersal);
    let (r, q) = row(&apart);
    assert_eq!(r, Structure::Dispersal, "flying apart (H) is not dispersal");
    // Every pair: d = 50√3, |Δv| = 2√3, μ = 1/6, K = 1, U = (1/9)/(50√3): K/U = 450√3.
    close(q, 450.0 * 3f64.sqrt(), "flying apart's smallest pair K/U");
    for (q_out, label) in [
        (2.0, Motion::He),
        (1.0, Motion::Pe),
        (0.5, Motion::Excursion),
    ] {
        let s = binary_plus_escaper(1.0, 0.0, 50.0, q_out);
        assert_eq!(classify(&at(&s), &cfg(), &Previous::none()), label);
        let (r, q) = row(&s);
        assert_eq!(r, Structure::Hierarchy, "{label:?} is not hierarchy");
        close(q, q_out, &format!("{label:?}'s outer K/U"));
    }
    let transient = equilateral_state(0.577, 2.0);
    let (r, q) = row(&transient);
    assert_eq!(r, Structure::Interacting, "transient is not interacting");
    // K = 3 · ½ · ⅓ · 4 = 2, |U| = 3 · (1/9)/(0.577√3): K/|U| = 6 · 0.577 · √3.
    close(q, 6.0 * 0.577 * 3f64.sqrt(), "transient's total K/|U|");
    let rest = equilateral_state(0.577, 0.0);
    let (r, q) = row(&rest);
    assert_eq!(r, Structure::Interacting, "B-so-far is not interacting");
    assert!(q.abs() <= Q_REL, "B-so-far at rest: q = {q:e}, not 0");
    let stable = ma_hierarchy(0.0);
    assert_eq!(
        classify(&at(&stable), &cfg(), &Previous::none()),
        Motion::BStable
    );
    let (r, q) = row(&stable);
    assert_eq!(r, Structure::Hierarchy, "B-stable is not hierarchy");
    // Body 2 at 10 from the pair's centre moving √0.1 across: μ = 2/9, K = 1/90, U = 1/45, K/U = ½.
    close(q, 0.5, "B-stable's outer K/U");
    // An OS-candidate: the state at rest with I_max = 2 · os_ratio · I.
    let mut os = at(&rest);
    os.i_max = 2.0 * cfg().os_ratio * inertia(&os.m, &os.r);
    assert_eq!(
        classify(&os, &cfg(), &Previous::none()),
        Motion::OsCandidate
    );
    assert_eq!(
        f(&os).0,
        Structure::Oscillatory,
        "OS-candidate is not oscillatory"
    );

    // §5.5's precedence, each overlap state with ρ above h_far.
    // The review's transient: pair unbound, body 2 at ρ = 10 receding with outer K/U = ½, E > 0.
    let t = transient_overlap();
    let ph = at(&t);
    assert_eq!(classify(&ph, &cfg(), &Previous::none()), Motion::Transient);
    assert!(ph.e > 0.0);
    let (r, q) = f(&ph);
    assert_eq!(
        r,
        Structure::Interacting,
        "the transient overlap state is not interacting"
    );
    assert!(
        q > 1.0,
        "the transient overlap state reads q = {q} (E = {:.4}), not above 1",
        ph.e
    );
    // A B-so-far whose outer body approaches with outer K/U = 2.
    let approaching = reversed(&binary_plus_escaper(1.0, 0.0, 50.0, 2.0));
    let ph = at(&approaching);
    assert_eq!(classify(&ph, &cfg(), &Previous::none()), Motion::BSoFar);
    let (r, q) = f(&ph);
    assert_eq!(
        r,
        Structure::Interacting,
        "the approaching B-so-far is not interacting"
    );
    assert!(
        q < 1.0,
        "the approaching B-so-far reads q = {q}, not below 1"
    );
    // The same state as an OS-candidate.
    let mut ph_os = ph;
    ph_os.i_max = 2.0 * cfg().os_ratio * inertia(&ph.m, &ph.r);
    assert_eq!(
        classify(&ph_os, &cfg(), &Previous::none()),
        Motion::OsCandidate
    );
    let (r, q) = f(&ph_os);
    assert_eq!(
        r,
        Structure::Oscillatory,
        "the far OS-candidate is not oscillatory"
    );
    assert!(q < 1.0, "the far OS-candidate reads q = {q}, not below 1");
    // A receding B-stable past h_far (§12.2 item 10's state).
    let receding = ma_hierarchy(0.05);
    let ph = at(&receding);
    assert_eq!(classify(&ph, &cfg(), &Previous::none()), Motion::BStable);
    let (r, q) = f(&ph);
    assert_eq!(r, Structure::Hierarchy);
    assert!(q < 1.0, "the receding B-stable reads q = {q}, not below 1");
    // The two-stage H: dispersal, q ≥ 1 − ε.
    for (name, s) in [
        ("two-stage H", two_stage()),
        ("co-moving two-stage H", two_stage_comoving()),
    ] {
        let ph = at(&s);
        assert_eq!(
            classify(&ph, &cfg(), &Previous::none()),
            Motion::H,
            "{name}"
        );
        let (r, q) = f(&ph);
        assert_eq!(r, Structure::Dispersal, "the {name} is not dispersal");
        assert!(
            q >= 1.0 - eps(),
            "the {name} reads q = {q}, below the parabolic band"
        );
    }
    // The co-moving one's q is the smaller of the inner pair's K/U (2) and the outer body's.
    let s = two_stage_comoving();
    let outer = s.outer(2, 0, 1);
    close(
        f(&at(&s)).1,
        2f64.min(outer.k / outer.u),
        "the co-moving two-stage H's q",
    );
}

#[test]
fn chazy_structure_rows_and_precedence() {
    check_rows(&rows);
}

negative_control!(
    chazy_structure_rows_and_precedence,
    "§5.5 taken literally (hierarchy wherever ρ > h_far) draws the transient overlap state dark",
    expected = "the transient overlap state is not interacting",
    check_rows(&literal)
);

#[test]
fn chazy_q_each_row_against_its_hand_computed_k_over_u() {
    check_rows(&rows);
}

negative_control!(
    chazy_q_each_row_against_its_hand_computed_k_over_u,
    "the dispersal row's q as the smallest of the three pairs alone draws the co-moving two-stage H dark",
    expected = "the co-moving two-stage H reads q",
    check_rows(&smallest_pair)
);

/// For interacting states, `q > 1` exactly when `E > 0` (`E` the label's, `E_0`), on a sweep of the equilateral
/// state's speed through `E = 0`, and on a drifted state whose live energy has the other sign from `E_0`.
fn check_interacting_sign(f: Rows) {
    let mut seen = [false; 2];
    for n in 0..=40 {
        let v = 0.5 + 0.025 * f64::from(n);
        let ph = at(&equilateral_state(0.577, v));
        let (r, q) = f(&ph);
        assert_eq!(r, Structure::Interacting);
        assert_eq!(q > 1.0, ph.e > 0.0, "speed {v}: q = {q}, E = {:e}", ph.e);
        seen[usize::from(ph.e > 0.0)] = true;
    }
    assert_eq!(seen, [true, true], "the sweep does not cross E = 0");
    // A drift: E_0 = +10⁻³, the live state's energy −10⁻³.
    let s = equilateral_state(0.577, 0.0);
    let u = -s.energy();
    let v = (2.0 * (u - 1e-3) / 1.0).sqrt();
    let live = equilateral_state(0.577, v);
    let mut ph = at(&live);
    assert!(ph.e < 0.0);
    ph.e = 1e-3;
    let (r, q) = f(&ph);
    assert_eq!(r, Structure::Interacting);
    assert!(
        q > 1.0,
        "the drifted transient (E_0 > 0, live E < 0) reads q = {q}, not above 1"
    );
}

#[test]
fn chazy_q_interacting_is_above_one_exactly_when_e_is_positive() {
    check_interacting_sign(&rows);
}

negative_control!(
    chazy_q_interacting_is_above_one_exactly_when_e_is_positive,
    "q from the live kinetic energy reads a drifted transient below 1",
    expected = "the drifted transient",
    check_interacting_sign(&live_kinetic)
);

/// The read side's accessors are the rows: `chazy_structure` and `chazy_q` through the stored inputs (level 0, `R_0`
/// 1, so `I_max = I_0 = 1`) equal [`rows`] on the read side's own playhead, bit for bit, on every fixture.
fn check_accessors(structure: fn(&State) -> u32) {
    for (name, s) in fixtures() {
        // The read side's playhead: velocities `p/m` from the stored momenta, `I_max = I_0 = M·R_0² = 1` at level 0.
        let ph = Playhead::from_momenta(s.m, s.r, s.p(), s.energy(), 1.0, 1.0);
        let (r, q) = rows(&ph);
        assert_eq!(
            structure(&s),
            r.code(),
            "{name}: chazy_structure is not its row"
        );
        let read = chazy_q(s.m, s.r, s.p(), s.energy(), 1.0, 0);
        assert_eq!(
            read.to_bits(),
            q.to_bits(),
            "{name}: chazy_q is not its row's q"
        );
    }
}

fn fixtures() -> Vec<(&'static str, State)> {
    vec![
        ("H", equilateral_state(50.0, 2.0)),
        ("HE", binary_plus_escaper(1.0, 0.0, 50.0, 2.0)),
        ("PE", binary_plus_escaper(1.0, 0.0, 50.0, 1.0)),
        ("excursion", binary_plus_escaper(1.0, 0.0, 50.0, 0.5)),
        ("eccentric HE", binary_plus_escaper(1.0, 0.95, 50.0, 2.0)),
        ("two-stage H", two_stage()),
        ("co-moving two-stage H", two_stage_comoving()),
        ("transient", equilateral_state(0.577, 2.0)),
        ("B-so-far", equilateral_state(0.577, 0.0)),
        ("B-stable", ma_hierarchy(0.0)),
        ("receding B-stable", ma_hierarchy(0.05)),
        ("transient overlap", transient_overlap()),
        (
            "approaching B-so-far",
            reversed(&binary_plus_escaper(1.0, 0.0, 50.0, 2.0)),
        ),
    ]
}

#[test]
fn chazy_structure_the_read_side_is_the_rows() {
    check_accessors(|s| chazy_structure(s.m, s.r, s.p(), s.energy(), 1.0, 0));
}

negative_control!(
    chazy_structure_the_read_side_is_the_rows,
    "a structure read from the label's code (not folded) fails",
    expected = "chazy_structure is not its row",
    check_accessors(|s| kernel::chazy::chazy_class(s.m, s.r, s.p(), s.energy(), 1.0, 0))
);

// ── chazy_escaper (§5.6) ───────────────────────────────────────────────────────────────────────────

/// The escaper computed here: the largest `ρ_b = d_out(b) / d_pair(i, j)` from `State`'s own two-body energies, the
/// lowest index on a tie.
fn escaper_here(s: &State) -> usize {
    let rho = |b: usize| {
        let (i, j) = match b {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
        s.outer(b, i, j).d / s.pair(i, j).d
    };
    (0..3).fold(0, |best, b| if rho(b) > rho(best) { b } else { best })
}

type Escaper = fn(&State) -> u32;

fn read_escaper(s: &State) -> u32 {
    chazy_escaper(s.m, s.r)
}

fn check_escaper(f: Escaper) {
    let he = binary_plus_escaper(1.0, 0.0, 50.0, 2.0);
    assert_eq!(f(&he), 2, "binary_plus_escaper's escaper is not body 2");
    // The tie: equilateral at rest, scale 1, its three ratios bitwise equal.
    let tie = equilateral_tie();
    let ph = at(&tie);
    let rho = [0, 1, 2].map(|b| kernel::chazy::hierarchy_ratio(&ph, b).to_bits());
    assert!(
        rho[0] == rho[1] && rho[1] == rho[2],
        "the tie fixture's ratios are not bitwise equal: {rho:x?}"
    );
    assert_eq!(f(&tie), 0, "the exact tie does not go to the lowest index");
    for (name, s) in fixtures() {
        assert_eq!(
            f(&s) as usize,
            escaper_here(&s),
            "{name}: the escaper is not the most separated body"
        );
    }
}

/// An equilateral state at rest, scale 1, built to tie exactly: the equilateral directions themselves (§12.1), centred
/// on the origin, with no centre-of-mass shift (`State::com` would bring in a rounding that breaks the tie). The
/// three ratios' bitwise equality is asserted where it is read; a triangle `(0, 0)`, `(2, 0)`, `(1, √3)` does not tie
/// (body 2's ratio one ulp below, measured), so the fixture is the construction, not the shape alone.
fn equilateral_tie() -> State {
    State {
        m: THIRDS,
        r: validation::chazy::equilateral(),
        v: [[0.0; 2]; 3],
    }
}

#[test]
fn chazy_escaper_the_body_most_separated_from_its_pair() {
    check_escaper(read_escaper);
}

negative_control!(
    chazy_escaper_the_body_most_separated_from_its_pair,
    "an escaper that keeps the last of equal ratios (`>=`) fails the tie",
    expected = "the exact tie does not go to the lowest index",
    check_escaper(|s| {
        let ph = at(s);
        let rho = [0, 1, 2].map(|b| kernel::chazy::hierarchy_ratio(&ph, b));
        let mut best = 0;
        if rho[1] >= rho[best] {
            best = 1;
        }
        if rho[2] >= rho[best] {
            best = 2;
        }
        best as u32
    })
);

/// Relabelling the bodies moves the escaper with them: binary_plus_escaper's escaper, old body 2, moved to index 0 and
/// then to index 1 (the control's state whose most separated body is 1).
fn check_relabelled(f: Escaper) {
    let he = binary_plus_escaper(1.0, 0.0, 50.0, 2.0);
    for (perm, want) in [([0, 1, 2], 2), ([2, 0, 1], 0), ([0, 2, 1], 1)] {
        assert_eq!(
            f(&relabelled(&he, perm)),
            want,
            "relabelled {perm:?}: the escaper is not body {want}"
        );
    }
}

#[test]
fn chazy_escaper_relabelled_follows_the_body() {
    check_relabelled(read_escaper);
}

negative_control!(
    chazy_escaper_relabelled_follows_the_body,
    "an escaper that names body 2 by default fails the relabelled state",
    expected = "relabelled",
    check_relabelled(|_| 2)
);

// ── chazy_qk (§5.6) ────────────────────────────────────────────────────────────────────────────────

type Qk = fn(&State) -> [f64; 3];

fn read_qk(s: &State) -> [f64; 3] {
    chazy_qk(s.m, s.r, s.p())
}

fn check_qk(f: Qk) {
    for q_out in [2.0, 1.0, 0.5] {
        let s = binary_plus_escaper(1.0, 0.0, 50.0, q_out);
        close(
            f(&s)[2],
            q_out,
            &format!("chazy_qk[2] at outer K/U = {q_out}"),
        );
    }
    for (name, s) in fixtures() {
        let qk = f(&s);
        assert!(
            qk.iter().all(|&q| q >= 0.0),
            "{name}: a negative chazy_qk {qk:?}"
        );
        // Each against `State`'s own outer energies.
        for (b, &got) in qk.iter().enumerate() {
            let (i, j) = match b {
                0 => (1, 2),
                1 => (0, 2),
                _ => (0, 1),
            };
            let o = s.outer(b, i, j);
            let want = o.k / o.u;
            assert!(
                (got - want).abs() <= Q_REL * want.max(1.0),
                "{name}: chazy_qk[{b}] = {got} against {want}"
            );
        }
        let ph = at(&s);
        let label = classify(&ph, &cfg(), &Previous::none());
        if matches!(label, Motion::He | Motion::Pe) {
            let e = read_escaper(&s) as usize;
            let q = chazy_q(s.m, s.r, s.p(), s.energy(), 1.0, 0);
            assert_eq!(
                qk[e].to_bits(),
                q.to_bits(),
                "{name}: chazy_qk[chazy_escaper] {} is not chazy_q {q}",
                qk[e]
            );
        }
    }
}

#[test]
fn chazy_qk_each_body_against_the_other_two() {
    check_qk(read_qk);
}

negative_control!(
    chazy_qk_each_body_against_the_other_two,
    "q_k read against the pair's own K/U, not the outer body's, fails",
    expected = "chazy_qk[2] at outer K/U",
    check_qk(|s| {
        let ph = at(s);
        [0, 1, 2].map(|b| {
            let (i, j) = others(b);
            let x = pair_kup(&ph, i, j);
            x.k / x.u
        })
    })
);

// ── The escape freeze (§7) ─────────────────────────────────────────────────────────────────────────

/// Every Chazy field of a stored sample, as bits: class, final, status, structure, q, escaper, qk.
fn fields_of(s: &SimStateBaseOf<f64>, m: [f64; 3], r_0: f64) -> [u64; 9] {
    let (state, code) = (sd_state(s.packed_a), sd_I_max(s.packed_a));
    let qk = chazy_qk(m, s.r, s.p);
    [
        u64::from(kernel::chazy::chazy_class(m, s.r, s.p, s.E_0, r_0, code)),
        u64::from(chazy_final(state, m, s.r, s.p, s.E_0, r_0, code)),
        u64::from(chazy_status(state, m, s.r, s.p, s.E_0, r_0, code)),
        u64::from(chazy_structure(m, s.r, s.p, s.E_0, r_0, code)),
        chazy_q(m, s.r, s.p, s.E_0, r_0, code).to_bits(),
        u64::from(chazy_escaper(m, s.r)),
        qk[0].to_bits(),
        qk[1].to_bits(),
        qk[2].to_bits(),
    ]
}

/// The freeze fixture: a circular pair (`a = 1`) and the third body at 50, receding with outer `K/U = 1.03`, inside the
/// parabolic band. The default rule fires at macro-step 40 on a state the class reads PE; marched on with the stop
/// option off, the outer ratio leaves the band and the class at the horizon reads HE (measured), so the control fails
/// on the class itself, not only on `q`.
fn freeze_fixture() -> State {
    binary_plus_escaper(1.0, 0.0, 50.0, 1.03)
}

const FREEZE_DT: f64 = 0.01;
const FREEZE_STEPS: u32 = 20_000;

/// Pass criterion, written before the run: under the stop option the march stops at escape fire, and at every later
/// playhead (every 500th macro-step from the fire step to the horizon) the stored sample's Chazy fields equal, bit for
/// bit, the fields at the fire step; the sample is `escape` throughout. `stop` is the stop option the check marches
/// under.
fn check_freeze(stop: bool) {
    let s = freeze_fixture();
    let ic = ic(&s);
    let r_0 = (inertia(&s.m, &s.r) / 1.0).sqrt();
    let full = integrate_detecting_to::<Rk4, f64>(
        &ic,
        &key(FREEZE_DT, FREEZE_STEPS),
        CollisionRadius::none(),
        FREEZE_STEPS,
        true,
    );
    assert_eq!(
        sd_state(full.packed_a),
        STATE_ESCAPE,
        "the freeze fixture does not escape"
    );
    let fire = tm_t_end_step(full.times);
    assert!(
        fire < FREEZE_STEPS / 2,
        "the escape fires at step {fire}, too late to watch"
    );
    let at_fire = fields_of(&full, s.m, r_0);
    assert_eq!(
        at_fire[0],
        u64::from(Motion::Pe.code()),
        "the class at fire is not PE"
    );
    let mut k = fire;
    let mut later = Vec::new();
    while k <= FREEZE_STEPS {
        let sample = integrate_detecting_to::<Rk4, f64>(
            &ic,
            &key(FREEZE_DT, FREEZE_STEPS),
            CollisionRadius::none(),
            k,
            stop,
        );
        later.push((k, sd_state(sample.packed_a), fields_of(&sample, s.m, r_0)));
        k += 500;
    }
    // The class first at every playhead, then every field, then the label.
    for (k, _, now) in &later {
        assert_eq!(
            now[0], at_fire[0],
            "playhead {k} (fire at {fire}): chazy_class differs from its fire-time value"
        );
    }
    for (k, state, now) in &later {
        assert_eq!(
            now, &at_fire,
            "playhead {k} (fire at {fire}): a Chazy field differs from its fire-time value"
        );
        assert_eq!(*state, STATE_ESCAPE, "playhead {k}: not escape");
    }
    let checked = later.len();
    assert!(checked >= 10, "only {checked} playheads after the fire");
    println!(
        "freeze: fire at macro-step {fire} (t = {:.2}), class {}, {checked} later playheads checked",
        f64::from(fire) * FREEZE_DT,
        at_fire[0]
    );
}

#[test]
fn chazy_escape_freeze_the_fields_read_the_fire_time_state() {
    check_freeze(true);
}

negative_control!(
    chazy_escape_freeze_the_fields_read_the_fire_time_state,
    "with the stop option off the march runs on past the fire and the class moves from PE to HE",
    expected = "chazy_class differs from its fire-time value",
    check_freeze(false)
);
