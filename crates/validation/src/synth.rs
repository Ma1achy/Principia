//! Shared states in the three regimes a trajectory seldom visits, and the check that a list of states lies in its
//! regime: near a collision ([`near_collision`]), across the substep law's thresholds up to its cap ([`high_substep`])
//! and at the invariant charts' feasibility edge ([`feasibility_edge`]).
//!
//! **A generator is not its own evidence.** Each generator places its states by arithmetic of its own and the matching
//! check reads them back through the kernel's predicates, the ones the pipeline branches on (`Separations`, `classify`,
//! `n_sub`, `no_construction`), so a generator pointed elsewhere is reported, not believed. A check wants membership
//! and coverage: every state in the regime, and both sides of every edge the regime has.
//!
//! **The straddles are adjacent.** The two sides of an edge are two states one f32 apart in one coordinate, so the d²
//! or `K*` of each is within a few ulps of the edge. The near pair's first body sits at the origin, where the f32
//! spacing is finest, so these states are not centred on their centre of mass; every quantity the step reads is a
//! difference of positions.
//!
//! **Domain** (pitfalls §10): inputs to one decode or one step. A state here need not lie on any trajectory.

use kernel::chart::invariant::InvariantBlocks;
use kernel::decode::invariant::no_construction;
use kernel::decode::{decode_with, Degenerate, Latent};
use kernel::detect::collision::{classify, CollisionRadius, Separations};
use kernel::driver::nsub_table::{threshold_bits, NsubConfig};
use kernel::driver::substep::{n_sub, NsubThresholds};
use kernel::generated::links::Defaults;
use kernel::occupant::State;
use kernel::stage::parity::DecodeIn;

use crate::decode_f32_eps::SplitMix;

/// How many f32 values a d² on an edge's two sides may lie from it.
pub const EDGE_ULPS: u32 = 8;

/// The near-collision regime: the closest pair's `d²` within this factor of `r_coll²`, either way.
pub const COLLISION_BAND: f32 = 32.0;

/// The feasibility edge's regime: `|2K*/L_z² − 1|` at most this.
pub const FEASIBILITY_BAND: f64 = 1.0 / 2048.0;

/// The next f32 away from zero.
fn out(x: f32) -> f32 {
    f32::from_bits(x.to_bits() + 1)
}

/// The next f32 towards zero.
fn inward(x: f32) -> f32 {
    f32::from_bits(x.to_bits() - 1)
}

/// The number of f32 values between two positive finite f32s.
fn ulps(a: f32, b: f32) -> u32 {
    a.to_bits().abs_diff(b.to_bits())
}

/// The kernel's `d²` of a pair whose offset is `v`, its first body at the origin.
fn d2_of(v: [f32; 2]) -> f32 {
    Separations::of(&[[0.0, 0.0], v, [8.0, 8.0]]).d2[2]
}

/// Two offsets one f32 apart in their larger coordinate, the kernel's `d²` of the first below `d2` and of the second not.
fn straddle(d2: f32, rng: &mut SplitMix) -> [[f32; 2]; 2] {
    let a = f64::from(rng.uniform(-0.75, 0.75));
    let turn = rng.next_u64();
    let s = f64::from(d2).sqrt();
    let sign = if turn & 1 == 0 { 1.0 } else { -1.0 };
    let (x, y) = ((sign * s * a.cos()) as f32, (s * a.sin()) as f32);
    let at = |x: f32| if turn & 2 == 0 { [x, y] } else { [y, x] };
    let mut far = x;
    while d2_of(at(far)) < d2 {
        far = out(far);
    }
    while d2_of(at(inward(far))) >= d2 {
        far = inward(far);
    }
    [at(inward(far)), at(far)]
}

/// The state whose pair `pair` (the side opposite body `pair`) has the offset `v`, body `pair` at `third`, the masses
/// drawn from `[0.1, 0.6]` and the momenta from `[−0.5, 0.5]`, body `pair`'s minus the other two's sum.
fn placed(pair: usize, v: [f32; 2], third: [f32; 2], rng: &mut SplitMix) -> State<f32> {
    let (a, b) = ((pair + 1) % 3, (pair + 2) % 3);
    let mut r = [[0.0; 2]; 3];
    r[b] = v;
    r[pair] = third;
    let mut p = [[0.0; 2]; 3];
    for x in [0, 1] {
        p[a][x] = rng.uniform(-0.5, 0.5);
        p[b][x] = rng.uniform(-0.5, 0.5);
        p[pair][x] = -(p[a][x] + p[b][x]);
    }
    let m = core::array::from_fn(|_| rng.uniform(0.1, 0.6));
    State { m, r, p }
}

/// A place for the third body, 0.8 to 1.2 from the origin.
fn far_body(rng: &mut SplitMix) -> [f32; 2] {
    let turn = f64::from(rng.uniform(-3.0, 3.0));
    let s = f64::from(rng.uniform(0.8, 1.2));
    [(s * turn.cos()) as f32, (s * turn.sin()) as f32]
}

/// `n` states in no regime: positions drawn from `[−1, 1]`, as [`placed`] draws the rest. The baseline the regimes'
/// numbers are read against.
pub fn ordinary(n: usize, seed: u64) -> Vec<State<f32>> {
    let mut rng = SplitMix::new(seed);
    (0..n)
        .map(|i| {
            let mut s = placed(i % 3, [0.0; 2], [0.0; 2], &mut rng);
            s.r = core::array::from_fn(|_| core::array::from_fn(|_| rng.uniform(-1.0, 1.0)));
            s
        })
        .collect()
}

// ── Near collision ─────────────────────────────────────────────────────────────────────────────────────────────

/// States whose closest pair's `d²` straddles `radius`'s `r_coll²`: each pair alone at `r_coll²` scaled by powers of 4
/// from 1/16 to 16, on both sides of each, and the three bodies together inside the radius and just outside it.
/// `r_coll = 0` is refused: no `d²` is below 0, so the regime is empty.
pub fn near_collision(radius: &CollisionRadius, seed: u64) -> Result<Vec<State<f32>>, String> {
    let r2 = radius.squared();
    // The plan does not say what straddling means at `r_coll = 0`; chose to refuse, since no d² is below 0.
    if r2 <= 0.0 {
        return Err("r_coll is 0: no d² is below it, so the near-collision regime is empty".into());
    }
    let mut rng = SplitMix::new(seed);
    let mut states = Vec::new();
    for pair in 0..3 {
        for scale in [0.0625, 0.25, 1.0, 4.0, 16.0] {
            for _ in 0..8 {
                for v in straddle(r2 * scale, &mut rng) {
                    let third = far_body(&mut rng);
                    states.push(placed(pair, v, third, &mut rng));
                }
            }
        }
    }
    for side in [0.5f32, 0.9, 1.1, 2.0] {
        for pair in 0..3 {
            let s = side * radius.root();
            let third = [0.5 * s, 0.866 * s];
            states.push(placed(pair, [s, 0.0], third, &mut rng));
        }
    }
    Ok(states)
}

/// Whether `states` lie in the near-collision regime of `radius` ([`COLLISION_BAND`]) and cover it: each pair within
/// [`EDGE_ULPS`] of `r_coll²` on both sides, each firing alone, the triple collision, and a state that does not fire.
pub fn in_near_collision(states: &[State<f32>], radius: &CollisionRadius) -> Result<(), String> {
    let r2 = radius.squared();
    let mut alone = [false; 4];
    let mut edge = [[false; 2]; 3];
    let mut quiet = false;
    for (i, s) in states.iter().enumerate() {
        let sep = Separations::of(&s.r);
        let (d2, pair) = sep.closest();
        if !(d2 * COLLISION_BAND >= r2 && d2 <= r2 * COLLISION_BAND) {
            return Err(format!(
                "state {i} lies outside the near-collision regime: its closest d² is {d2:e}, r_coll² is {r2:e}"
            ));
        }
        let v = classify(&sep, radius);
        if v.fired {
            alone[v.detail as usize] = true;
        } else {
            quiet = true;
        }
        if ulps(d2, r2) <= EDGE_ULPS {
            edge[pair as usize][usize::from(d2 >= r2)] = true;
        }
    }
    if alone == [true; 4] && edge == [[true; 2]; 3] && quiet {
        Ok(())
    } else {
        Err(format!(
            "the near-collision states leave part of the regime uncovered: verdicts 0 to 3 seen {alone:?}, not fired \
             {quiet}, each pair's two sides of r_coll² {edge:?}"
        ))
    }
}

// ── High substep count ─────────────────────────────────────────────────────────────────────────────────────────

/// States whose `d²` straddles every threshold that decides `N_sub` under `law`, so that every count from 1 to `N_max`
/// is taken, and states far below the last of them, at the cap. Refuses what the host's threshold generator refuses.
pub fn high_substep(law: &NsubConfig, seed: u64) -> Result<Vec<State<f32>>, String> {
    let bits = threshold_bits(law).map_err(|e| e.to_string())?;
    let mut rng = SplitMix::new(seed);
    let mut states = Vec::new();
    let deciding = &bits[..law.n_max as usize - 1];
    for (n, &b) in deciding.iter().enumerate() {
        for pair in [n % 3, (n + 1) % 3] {
            for v in straddle(f32::from_bits(b), &mut rng) {
                let third = far_body(&mut rng);
                states.push(placed(pair, v, third, &mut rng));
            }
        }
    }
    if let Some(&last) = deciding.last() {
        for k in 0..48 {
            let d2 = f32::from_bits(last) * 0.5f32.powi(1 + k / 3);
            let [v, _] = straddle(d2, &mut rng);
            let third = far_body(&mut rng);
            states.push(placed(k as usize % 3, v, third, &mut rng));
        }
    }
    Ok(states)
}

/// Whether `states` lie in the high-substep regime of `law`, each with `d²` within [`EDGE_ULPS`] of a deciding threshold
/// or at the cap, and cover it: both sides of every deciding threshold, and every `N_sub` from 1 to `N_max`.
pub fn in_high_substep(states: &[State<f32>], law: &NsubConfig) -> Result<(), String> {
    let bits = threshold_bits(law).map_err(|e| e.to_string())?;
    let table: NsubThresholds = kernel::driver::nsub_table::pack(&bits);
    let deciding = &bits[..law.n_max as usize - 1];
    let mut sides = vec![[false; 2]; deciding.len()];
    let mut counts = vec![false; law.n_max as usize];
    for (i, s) in states.iter().enumerate() {
        let d2 = Separations::of(&s.r).min();
        let n = n_sub(d2, &table, law.n_max);
        counts[n as usize - 1] = true;
        let mut at_edge = false;
        for (side, &b) in sides.iter_mut().zip(deciding) {
            if d2.to_bits().abs_diff(b) <= EDGE_ULPS {
                at_edge = true;
                side[usize::from(d2 >= f32::from_bits(b))] = true;
            }
        }
        if !at_edge && n != law.n_max {
            return Err(format!(
                "state {i} lies outside the high-substep regime: its d² is {d2:e}, at no threshold, and its N_sub is {n}"
            ));
        }
    }
    let edges = sides.iter().filter(|s| **s != [true; 2]).count();
    let missing = counts.iter().filter(|c| !**c).count();
    if edges == 0 && missing == 0 {
        Ok(())
    } else {
        Err(format!(
            "the high-substep states leave part of the regime uncovered: {edges} thresholds lack a side, {missing} of \
             the counts 1 to {} are not taken",
            law.n_max
        ))
    }
}

// ── The feasibility edge ───────────────────────────────────────────────────────────────────────────────────────

/// Decode inputs whose targets straddle `K* = L_z²/(2I)` at the moment of inertia `inertia`, 1 being the decode's: for
/// each `L_z`, the edge's f32, its neighbours out to three, and one a relative `2⁻¹²` away on each side. The `L_z` of
/// twelve bits or fewer have an exact `K*`, so the edge itself is among them; the rest have a rounded one.
pub fn feasibility_edge(inertia: f32, seed: u64) -> Vec<DecodeIn<f32>> {
    let mut rng = SplitMix::new(seed);
    let mut inputs = Vec::new();
    for j in 0..64 {
        let drawn = rng.uniform(0.02, 1.5);
        let size = if j % 2 == 0 {
            (drawn * 1024.0).ceil() / 1024.0
        } else {
            drawn
        };
        let lz = if j % 4 < 2 { size } else { -size };
        let edge = (f64::from(lz) * f64::from(lz) / (2.0 * f64::from(inertia))) as f32;
        let mut targets = vec![edge * (1.0 - 1.0 / 4096.0), edge * (1.0 + 1.0 / 4096.0)];
        targets.extend((0..7).map(|n| f32::from_bits(edge.to_bits() + n - 3)));
        for k_star in targets {
            let z = core::array::from_fn(|_| rng.uniform(-0.8, 0.8));
            inputs.push(DecodeIn { z, lz, k_star });
        }
    }
    inputs
}

/// Whether `inputs` lie at the feasibility edge ([`FEASIBILITY_BAND`], the moment of inertia 1 the decode's) and cover
/// it: targets within three f32 values of the edge on both sides, the exact edge, and, at f32, a refusal of exactly
/// those on the infeasible side.
pub fn in_feasibility_edge(inputs: &[DecodeIn<f32>]) -> Result<(), String> {
    let mut near = [false; 2];
    let mut exact = false;
    for (i, s) in inputs.iter().enumerate() {
        let (l, k) = (f64::from(s.lz), f64::from(s.k_star));
        let ratio = 2.0 * k / (l * l);
        if (ratio - 1.0).abs() > FEASIBILITY_BAND {
            return Err(format!(
                "input {i} lies outside the feasibility edge's regime: 2K*/L_z² is {ratio}"
            ));
        }
        let refused = no_construction(s.k_star, s.lz);
        let mut blocks = InvariantBlocks::<Defaults, f32>::new(s.lz, s.k_star);
        let tag = decode_with(&mut blocks, &Latent::new(s.z)).degenerate;
        if (tag == Some(Degenerate::InvariantInfeasible)) != refused {
            return Err(format!(
                "input {i}: the decode's tag is {tag:?} where the predicate's refusal is {refused}"
            ));
        }
        // Both products are exact in f64: 24-bit factors.
        exact |= 2.0 * k == l * l;
        let edge = (l * l / 2.0) as f32;
        if ulps(s.k_star, edge) <= 3 {
            near[usize::from(!refused)] = true;
        }
    }
    if exact && near == [true; 2] {
        Ok(())
    } else {
        Err(format!(
            "the feasibility-edge inputs leave part of the regime uncovered: infeasible and feasible beside the edge \
             {near:?}, the exact edge {exact}"
        ))
    }
}
