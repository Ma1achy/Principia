//! The read-back (`kernel::stage::branch_read`; TASK-M4-27; REQ-VAL-347): each word of a sample sits in its slot, as the
//! kernel's own branch functions give it, canon's and the decode's alike, the outcome what `canonicalise` and the decode
//! give; each chart reads what its decode reads; the entries write sample `i` and nothing past the last; and the branch
//! functions at the boundary of each comparison. Each test registers the control that must make it fail (R-176).

use kernel::canon::{
    axis_zero, canonicalise, com_frame, is_pole, mirror_input, mirror_side, pair_differences,
    scaled_rho, skip_input, skip_taken, step_runs, wide_state, Canonical, Wide,
};
use kernel::chart::burrau::{seed, seed_input, seed_of_sum};
use kernel::chart::invariant::Invariant;
use kernel::chart::mixed::{Alpha, ZMu1};
use kernel::chart::shape_sphere::{inverse_pole, inverse_pq};
use kernel::chart::{
    Burrau, BurrauThetaDeltaM, BurrauThetaK, BurrauThetaLz, Chart, ChartDecode, EuclidPlane,
    InvariantLE, InvariantLK, Latent, MassSimplex, Mixed, ShapeMap, ShapeSphere,
};
use kernel::decode::descriptor::{
    collision_pole, cross_dot, cross_zero, dot_negative, dot_zero, ratio_pole, virial_pole,
};
use kernel::decode::invariant::{
    alpha_sign, feasibility_input, feasibility_short, seed_of_sign, seed_word,
};
use kernel::decode::mass::{m01_input, m01_tiny};
use kernel::decode::{energy_norm, Ic};
use kernel::stage::branch_read::{self as read, BranchChart, BranchVariant, ChartReads};
use kernel::stage::decode::{
    write_ic, BurrauDefaults, BurrauThetaDeltaMDefaults, BurrauThetaKDefaults,
    BurrauThetaLzDefaults, DecodeUniforms, EuclidPlaneDefaults, InvariantLEDefaults,
    InvariantLKDefaults, LatentDefaults, LatentStickBreaking, MassSimplexDefaults,
    MixedMu1AlphaDefaults, ShapeSphereDefaults, UniformChart,
};
use spirv_std::glam::UVec3;
use validation::negative_control;

const SENTINEL: u32 = 0xDEAD_BEEF;

fn word(c: bool) -> u32 {
    u32::from(c)
}

fn wides(v: &[Wide<f32>]) -> Vec<u32> {
    v.iter().flat_map(|w| w.words()).collect()
}

/// `m`, `r`, as `write_canon`'s doc lays them out, read from the kernel's functions one by one.
fn expected_canon(m: [f32; 3], r: [[f32; 2]; 3]) -> Vec<u32> {
    let mut e: Vec<u32> = m.iter().map(|x| x.to_bits()).collect();
    e.extend(r.iter().flatten().map(|x| x.to_bits()));
    let (mass, x) = wide_state(m, r);
    let (rho, d02, d12) = pair_differences(x);
    let (turned, clamped) = scaled_rho(rho);
    let coincident = clamped & is_pole(d02);
    let (_, i) = com_frame(mass, x);
    let sides = skip_input(i);
    e.push(word(step_runs(skip_taken(sides), coincident)));
    e.extend(wides(&sides));
    e.push(word(coincident));
    e.push(word(coincident));
    e.extend(wides(&[rho[0], rho[1], d02[0], d02[1]]));
    e.push(word(clamped));
    e.extend(wides(&rho));
    let input = mirror_input(mass, rho, d02, d12);
    e.push(word(mirror_side(&input)));
    e.extend(wides(&[input.c, input.left, input.right]));
    for t in turned {
        e.push(word(axis_zero(t)));
        e.push(t.to_bits());
    }
    e
}

/// States that raise each of canon's branches, and one that raises none.
fn states() -> Vec<Ic<f32>> {
    let m = [0.2, 0.3, 0.5];
    let p = [[0.1, -0.2], [0.3, 0.05], [-0.4, 0.15]];
    let of = |r| Ic { m, r, p };
    vec![
        of([[0.1, 0.2], [0.7, -0.3], [-0.4, 0.9]]),
        of([[0.1, 0.9], [0.7, 0.3], [-0.4, -0.2]]),
        of([[0.3, 0.3], [0.3, 0.3], [0.3, 0.3]]),
        of([[0.3, 0.3], [0.3, 0.3], [-0.5, 0.75]]),
        of([[0.0, 0.0], [0.5, 0.0], [0.25, 0.5]]),
        of([[0.0, 0.0], [0.0, 0.5], [0.5, 0.25]]),
        canonicalise(&of([[0.1, 0.2], [0.7, -0.3], [-0.4, 0.9]])).ic,
    ]
}

/// `words` written from word `at` of a buffer of sentinels, every word outside them left alone.
fn check_written(n: usize, write: impl Fn(&mut [u32], usize)) -> Vec<u32> {
    let at = 2 * n + 3;
    let mut out = vec![SENTINEL; 4 * n + 6];
    write(&mut out, at);
    let inside = at..at + n;
    for (k, w) in out.iter().enumerate() {
        if !inside.contains(&k) {
            assert_eq!(
                *w, SENTINEL,
                "a word was written outside its sample, at {k}"
            );
        }
    }
    out[inside].to_vec()
}

fn check_canon(transform: fn(&mut [u32])) {
    for s in states() {
        let mut got = check_written(read::CANON_WORDS, |o, at| {
            read::write_canon(o, at, s.m, s.r)
        });
        transform(&mut got);
        assert_eq!(
            got,
            expected_canon(s.m, s.r),
            "canon's words are not in their slots"
        );
        let c: Canonical<f32> = canonicalise(&s);
        assert_eq!(got[read::SKIP_AT], word(c.rescaled), "the skip's outcome");
        assert_eq!(got[read::GUARD_AT], word(c.clamped), "the guard's outcome");
        assert_eq!(
            got[read::MIRROR_AT],
            word(c.mirrored),
            "the mirror's outcome"
        );
    }
    let flags = |f: fn(&Canonical<f32>) -> bool| states().iter().any(|s| f(&canonicalise(s)));
    assert!(flags(|c| c.rescaled) && flags(|c| c.clamped) && flags(|c| c.mirrored));
    assert!(flags(|c| !c.rescaled), "no state skipped step 0");
    let zero = |at: usize| {
        states().iter().any(|s| {
            let mut w = vec![0; read::CANON_WORDS];
            read::write_canon(&mut w, 0, s.m, s.r);
            w[at] == 1
        })
    };
    assert!(zero(read::RHO_X_AT) && zero(read::RHO_Y_AT) && zero(read::COINCIDENCE_AT));
}

#[test]
fn branch_read_canon_words_sit_in_their_slots() {
    check_canon(|_| {});
}

negative_control!(
    branch_read_canon_words_sit_in_their_slots,
    "two of the skip's words swapped",
    expected = "canon's words are not in their slots",
    check_canon(|w| w.swap(read::SKIP_AT + 1, read::SKIP_AT + 3))
);

#[test]
fn branch_read_the_layout_is_the_declared_one() {
    let shapes = [
        (read::SKIP_AT, 9),
        (read::COINCIDENCE_AT, 15),
        (read::GUARD_AT, 24),
        (read::MIRROR_AT, 29),
        (read::RHO_X_AT, 36),
        (read::RHO_Y_AT, 38),
        (read::CANON_WORDS, 40),
        (read::CANON_OPERANDS, 9),
        (read::SKIP_BITS, 5),
        (read::COINCIDENCE_BITS, 8),
        (read::GUARD_BITS, 4),
        (read::MIRROR_BITS, 6),
        (read::AXIS_BITS, 1),
        (read::M01_AT, 0),
        (read::FEASIBILITY_AT, 8),
        (read::ALPHA_SEED_AT, 17),
        (read::NU_SEED_AT, 22),
        (read::ENERGY_INFEASIBLE_AT, 27),
        (read::ENERGY_UNSCALABLE_AT, 29),
        (read::RATIO_POLE_AT, 31),
        (read::COLLISION_POLE_AT, 33),
        (read::VIRIAL_POLE_AT, 35),
        (read::CROSS_ZERO_AT, 37),
        (read::DOT_ZERO_AT, 39),
        (read::DOT_NEGATIVE_AT, 41),
        (read::SPHERE_POLE_AT, 43),
        (read::DECODE_WORDS, 45),
    ];
    for (k, (got, want)) in shapes.into_iter().enumerate() {
        assert_eq!(got, want, "the layout moved, entry {k}");
    }
    assert_eq!(read::M01_SHAPE, (2, 4));
    assert_eq!(read::FEASIBILITY_SHAPE, (2, 5));
    assert_eq!(read::ALPHA_SEED_SHAPE, (1, 2));
    assert_eq!(read::NU_SEED_SHAPE, (1, 2));
}

negative_control!(
    branch_read_the_layout_is_the_declared_one,
    "a word past the layout",
    expected = "the layout moved",
    assert_eq!(read::CANON_WORDS + 1, 40, "the layout moved")
);

fn check_m01(m0: f32, m1: f32) {
    let got = check_written(8, |o, at| read::write_m01(o, at, m0, m1));
    let sides = m01_input(m0, m1);
    let mut e = vec![1, word(m01_tiny(sides)), m0.to_bits(), m1.to_bits()];
    e.extend(wides(&sides));
    assert_eq!(got, e, "M01_TINY's slot");
}

#[test]
fn branch_read_slots_of_the_decode_branches() {
    check_m01(0.25, 0.5);
    check_m01(0.0, 0.0);
    for (k, lz) in [
        (1.0, 1.0),
        (1.0, 2.0),
        (0.0, 0.0),
        (0.0, 0.5),
        (-0.5, 1.0),
        (0.125, 0.5),
    ] {
        let got = check_written(9, |o, at| read::write_feasibility(o, at, k, lz));
        let f = feasibility_input(1.0, k, lz);
        let e = vec![
            1,
            word(feasibility_short(&f)),
            k.to_bits(),
            lz.to_bits(),
            word(f.flat),
            word(f.negative),
            word(f.moving),
            f.e as u32,
            f.columns.to_bits(),
        ];
        assert_eq!(got, e, "INVARIANT_INFEASIBLE's slot");
    }
    for z in [1.0, -1.0, 0.0, -0.0] {
        let got = check_written(5, |o, at| read::write_alpha_seed(o, at, z));
        let (negative, zero) = alpha_sign(z);
        let seeded = seed_word(Some(seed_of_sign(negative, zero)));
        let e = vec![1, seeded, z.to_bits(), word(negative), word(zero)];
        assert_eq!(got, e, "the seed on z_α's slot");
    }
    for nu in [0.25, 1.0 / 3.0, 0.5] {
        let got = check_written(5, |o, at| read::write_nu_seed(o, at, nu));
        let (s, sum) = seed_input(nu);
        let e = vec![
            1,
            seed_word(Some(seed_of_sum(s, sum))),
            nu.to_bits(),
            s.to_bits(),
            sum.to_bits(),
        ];
        assert_eq!(got, e, "the seed on ν's slot");
    }
}

negative_control!(
    branch_read_slots_of_the_decode_branches,
    "the wrong masses read back",
    expected = "M01_TINY's slot",
    {
        let got = check_written(8, |o, at| read::write_m01(o, at, 0.25, 0.5));
        let mut e = vec![1, 0, 0.5f32.to_bits(), 0.25f32.to_bits()];
        e.extend(wides(&m01_input(0.25f32, 0.5)));
        assert_eq!(got, e, "M01_TINY's slot");
    }
);

/// The words `write_decode` gives `chart` at `(u, v)`, read from the kernel's functions slot by slot.
fn expected_decode<V: BranchVariant>(
    chart: &V::Chart,
    (u, v): (f32, f32),
    e_star: Option<f32>,
    reads: &ChartReads,
) -> Vec<u32> {
    let mut e = vec![0; read::DECODE_WORDS];
    let forbids = chart.forbids_energy_normalisation();
    let (d, energy) =
        energy_norm::apply_shared(chart.decode_at::<V::Links, f32>(u, v), e_star, forbids);
    read::write_m01(&mut e, read::M01_AT, d.ic.m[0], d.ic.m[1]);
    if reads.construction {
        read::write_feasibility(&mut e, read::FEASIBILITY_AT, reads.k, reads.lz);
    }
    if reads.alpha_seed {
        read::write_alpha_seed(&mut e, read::ALPHA_SEED_AT, reads.z_alpha);
    }
    if reads.nu_seed {
        read::write_nu_seed(&mut e, read::NU_SEED_AT, reads.nu);
    }
    let mut other = |at: usize, c: bool| {
        e[at] = 1;
        e[at + 1] = word(c);
    };
    if energy != energy_norm::Outcome::Off {
        let infeasible = energy == energy_norm::Outcome::Infeasible;
        other(read::ENERGY_INFEASIBLE_AT, infeasible);
        other(
            read::ENERGY_UNSCALABLE_AT,
            energy == energy_norm::Outcome::Unscalable,
        );
    }
    let p = d.descriptor;
    other(read::RATIO_POLE_AT, ratio_pole(p.lambda_mag));
    other(read::COLLISION_POLE_AT, collision_pole(p.r_min_pair_0));
    other(read::VIRIAL_POLE_AT, virial_pole(p.V_0));
    let (cross, dot) = cross_dot(d.config.rho_t, d.config.lambda_t);
    other(read::CROSS_ZERO_AT, cross_zero(cross));
    other(read::DOT_ZERO_AT, dot_zero(dot));
    other(read::DOT_NEGATIVE_AT, dot_negative(dot));
    if reads.sphere {
        other(read::SPHERE_POLE_AT, inverse_pole(reads.pq[0], reads.pq[1]));
    }
    e
}

fn zeroed_decode<V: BranchVariant>(chart: &V::Chart, p: (f32, f32), e: Option<f32>) -> Vec<u32> {
    let n = read::DECODE_WORDS;
    let mut out = vec![0; 3 * n];
    read::write_decode::<V>(&mut out, n, chart, p, e);
    assert!(
        out[..n].iter().chain(&out[2 * n..]).all(|w| *w == 0),
        "outside its sample"
    );
    out[n..2 * n].to_vec()
}

const POINTS: [(f32, f32); 3] = [(0.5, 0.5), (0.25, 0.75), (0.875, 0.125)];

fn check_decode<V: BranchVariant>(chart: &V::Chart) {
    for p in POINTS {
        let reads = chart.reads(p.0, p.1);
        for e in [None, Some(0.0), Some(-0.3), Some(-40.0), Some(1.5)] {
            let got = zeroed_decode::<V>(chart, p, e);
            assert_eq!(
                got,
                expected_decode::<V>(chart, p, e, &reads),
                "the decode's words are not in their slots, {}",
                V::ENTRY
            );
            let raised = got[read::ENERGY_INFEASIBLE_AT] == 1;
            let forbidden = chart.forbids_energy_normalisation();
            assert_eq!(
                raised,
                e.is_some() && !forbidden && got[read::M01_AT + 1] == 0,
                "η_E's rows, {}",
                V::ENTRY
            );
        }
    }
}

fn invariant() -> Invariant<f32> {
    Invariant {
        z_cfg: [0.4, -0.6],
        z_mass: [0.3, -0.2],
        k_max: 2.0,
        gamma_k: 2.0,
    }
}

fn burrau() -> Burrau<f32> {
    Burrau {
        nu_range: [0.05, 0.95],
        k_max: 2.0,
        gamma_k: 2.0,
    }
}

fn strip_k() -> BurrauThetaK<f32> {
    BurrauThetaK {
        theta_range: [0.1, 1.4],
        k_max: 2.0,
        gamma_k: 2.0,
    }
}

fn strip_lz() -> BurrauThetaLz<f32> {
    BurrauThetaLz {
        theta_range: [0.1, 1.4],
        k: 1.0,
        lz_fraction: 0.5,
    }
}

fn sphere() -> ShapeSphere<f32> {
    ShapeSphere {
        map: ShapeMap::Spherical,
        n0: [0.0, 0.0, 1.0],
        e1: [1.0, 0.0, 0.0],
        e2: [0.0, 1.0, 0.0],
        scale: 1.2,
        inertia: 1.0,
        phase: 0.0,
        z_mass: [0.2, -0.3],
        z_mom: [0.1; 4],
    }
}

fn latent() -> Latent<f32> {
    let eight = |k: f32| core::array::from_fn(|i| k * (i as f32 - 3.5) / 8.0);
    Latent {
        z0: eight(1.0),
        q1: eight(0.5),
        q2: eight(-0.25),
    }
}

fn check_every_decode() {
    check_decode::<LatentDefaults>(&latent());
    check_decode::<LatentStickBreaking>(&latent());
    check_decode::<InvariantLEDefaults>(&InvariantLE(invariant()));
    check_decode::<InvariantLKDefaults>(&InvariantLK(invariant()));
    check_decode::<MassSimplexDefaults>(&MassSimplex {
        nu0: 0.5,
        eps_m: 1e-3,
    });
    check_decode::<BurrauDefaults>(&burrau());
    check_decode::<EuclidPlaneDefaults>(&EuclidPlane {
        m_range: [0.5, 2.0],
        nu_range: [0.1, 0.9],
    });
    check_decode::<MixedMu1AlphaDefaults>(&Mixed::<ZMu1, Alpha, f32>::new(
        core::array::from_fn(|i| i as f32 / 8.0 - 0.5),
        [-2.0, 2.0],
        [0.1, 1.4],
    ));
    check_decode::<BurrauThetaKDefaults>(&strip_k());
    check_decode::<BurrauThetaLzDefaults>(&strip_lz());
    check_decode::<BurrauThetaDeltaMDefaults>(&BurrauThetaDeltaM {
        theta_range: [0.1, 1.4],
        m_target: [0.25, 0.375, 0.375],
    });
    check_decode::<ShapeSphereDefaults>(&sphere());
}

#[test]
fn branch_read_decode_words_sit_in_their_slots() {
    check_every_decode();
}

negative_control!(
    branch_read_decode_words_sit_in_their_slots,
    "a row emitted from a chart that has none",
    expected = "the decode's words are not in their slots",
    {
        let chart = latent();
        let p = (0.5, 0.5);
        let mut reads = chart.reads(p.0, p.1);
        reads.sphere = true;
        assert_eq!(
            zeroed_decode::<LatentDefaults>(&chart, p, None),
            expected_decode::<LatentDefaults>(&chart, p, None, &reads),
            "the decode's words are not in their slots"
        );
    }
);

#[test]
fn branch_read_each_chart_reads_what_its_decode_reads() {
    let none = latent().reads(0.5, 0.5);
    assert!(!(none.construction | none.alpha_seed | none.nu_seed | none.sphere));
    let le = InvariantLE(invariant());
    for (chart, (u, v)) in [(&le, POINTS[0]), (&le, POINTS[1])] {
        let r = chart.reads(u, v);
        let t = le.0.targets::<f32>(u, v);
        assert!(r.construction && r.alpha_seed && !r.nu_seed && !r.sphere);
        assert_eq!((r.k, r.lz), (t.k, t.lz));
        assert_eq!(r.z_alpha, 0.4, "z_α is the frozen control");
    }
    let lk = InvariantLK(invariant());
    let r = lk.reads(0.25, 0.75);
    let t = lk.0.targets::<f32>(0.25, 0.75);
    assert!(r.construction && r.alpha_seed);
    assert_eq!((r.k, r.lz), (t.k, t.lz));

    let (u, v) = POINTS[1];
    let b = burrau();
    let r = b.reads(u, v);
    assert!(r.construction && r.nu_seed && !r.alpha_seed && !r.sphere);
    assert_eq!((r.k, r.lz, r.nu), (b.kinetic(v), 0.0, b.nu(u)));
    let s = strip_k();
    let r = s.reads(u, v);
    assert!(r.construction && r.nu_seed);
    assert_eq!((r.k, r.lz, r.nu), (s.kinetic(v), 0.0, s.nu(u)));
    let s = strip_lz();
    let r = s.reads(u, v);
    assert!(r.construction && r.nu_seed);
    assert_eq!((r.k, r.lz, r.nu), (s.kinetic(), s.lz(v), s.nu(u)));

    let s = sphere();
    let r = s.reads(u, v);
    assert!(r.sphere && !r.construction && !r.alpha_seed && !r.nu_seed);
    assert_eq!(r.pq, inverse_pq(s.point(u, v), s.inertia));
}

negative_control!(
    branch_read_each_chart_reads_what_its_decode_reads,
    "the inverse read at another point",
    expected = "the point the inverse was read at",
    {
        let s = sphere();
        assert_eq!(
            s.reads(0.25, 0.75).pq,
            inverse_pq(s.point(0.5, 0.5), s.inertia),
            "the point the inverse was read at"
        );
    }
);

/// `uv` and the words each entry should give it, sample by sample.
fn check_entry(run: fn(u32, &DecodeUniforms, &[f32], &mut [u32])) {
    let chart = latent();
    let uniforms = DecodeUniforms {
        chart: chart.uniform(),
        ..Default::default()
    };
    let uv = [0.5, 0.5, 0.25, 0.75, 0.125, 0.875];
    let n = read::DECODE_WORDS;
    let mut out = vec![0; 3 * n];
    run(1, &uniforms, &uv, &mut out);
    let want = zeroed_decode::<LatentDefaults>(&chart, (0.25, 0.75), None);
    assert_eq!(&out[n..2 * n], want, "sample 1 is not at its place");
    assert!(out[..n].iter().chain(&out[2 * n..]).all(|w| *w == 0));
    run(2, &uniforms, &uv, &mut out);
    let last = zeroed_decode::<LatentDefaults>(&chart, (0.125, 0.875), None);
    assert_eq!(&out[2 * n..], last);
    let before = out.clone();
    run(3, &uniforms, &uv, &mut out);
    assert_eq!(out, before, "past the last sample it wrote");
}

#[test]
fn branch_read_an_entry_writes_its_sample_alone() {
    check_entry(|i, u, uv, out| read::latent_defaults(UVec3::new(i, 0, 0), u, uv, out));

    let mut ic = vec![0; 2 * kernel::stage::decode::ic_words()];
    let s = states();
    write_ic(&mut ic, 0, &s[0]);
    write_ic(&mut ic, 1, &s[1]);
    let n = read::CANON_WORDS;
    let mut out = vec![0; 3 * n];
    read::canon(UVec3::new(1, 0, 0), &ic, &mut out);
    assert_eq!(out[n..2 * n], expected_canon(s[1].m, s[1].r));
    assert!(out[..n].iter().chain(&out[2 * n..]).all(|w| *w == 0));
    let before = out.clone();
    read::canon(UVec3::new(2, 0, 0), &ic, &mut out);
    assert_eq!(out, before, "past the last state it wrote");
    assert_eq!(
        LatentDefaults::READ_ENTRY,
        "stage_branch_read_latent_defaults"
    );
}

negative_control!(
    branch_read_an_entry_writes_its_sample_alone,
    "another variant's entry",
    expected = "sample 1 is not at its place",
    check_entry(|i, u, uv, out| read::latent_stick_breaking(UVec3::new(i, 0, 0), u, uv, out))
);

fn check_poles() {
    assert!(collision_pole(0.0f32) && collision_pole(-0.0f32));
    assert!(!collision_pole(f32::from_bits(1)));
    assert!(ratio_pole(0.0f32) && !ratio_pole(f32::from_bits(1)));
    assert!(cross_zero(-0.0f32) && !cross_zero(-f32::from_bits(1)));
    assert!(dot_zero(0.0f32) && !dot_zero(f32::from_bits(1)));
    assert!(dot_negative(-f32::from_bits(1)), "the least negative dot");
    assert!(!dot_negative(0.0f32) && !dot_negative(-0.0f32) && !dot_negative(1.0f32));
    assert!(virial_pole(f32::NEG_INFINITY) && virial_pole(f32::NAN));
    assert!(!virial_pole(f32::MAX) && !virial_pole(0.0f32));
}

#[test]
fn branch_read_poles_at_their_boundary() {
    check_poles();
    assert!(inverse_pole(0.0f32, 0.0) && inverse_pole(0.0f32, -0.0));
    assert!(!inverse_pole(f32::from_bits(1), 0.0) && !inverse_pole(0.0, f32::from_bits(1)));
    let [p, q] = inverse_pq([1.0f32, -0.0, 0.5], 2.0);
    assert_eq!(
        (p.to_bits(), q),
        (0.0f32.to_bits(), 0.5),
        "a −0 taken as +0"
    );
    let [p, q] = inverse_pq([1.0f32, 0.5, 0.25], 3.0);
    assert_eq!((p, q), (0.75, 0.375));
}

negative_control!(
    branch_read_poles_at_their_boundary,
    "the pole's own boundary moved",
    expected = "the least negative dot",
    assert!(dot_negative(f32::from_bits(1)), "the least negative dot")
);

fn check_canon_branches() {
    assert!(step_runs(false, false));
    assert!(!step_runs(true, false), "a skipped step ran");
    assert!(!step_runs(false, true) && !step_runs(true, true));
    assert!(axis_zero(0.0f32) && axis_zero(-0.0f32) && !axis_zero(f32::from_bits(1)));
}

#[test]
fn branch_read_canon_branches_at_their_boundary() {
    check_canon_branches();
}

negative_control!(
    branch_read_canon_branches_at_their_boundary,
    "a step that runs when skipped",
    expected = "a skip read as run",
    assert!(!step_runs(false, false), "a skip read as run")
);

fn check_seeds() {
    assert_eq!(
        seed(1.0f32 / 3.0),
        seed_of_sum(1.0f32, seed_input(1.0f32 / 3.0).1)
    );
    let (below, above) = (0.25f32, 0.5f32);
    let (s, e) = seed_input(below);
    assert_eq!(seed_of_sum(s, e), seed(below));
    let (s, e) = seed_input(above);
    assert_eq!(seed_of_sum(s, e), seed(above));
    assert_ne!(seed(below), seed(above), "the two sides of 3ν = 1");
    assert_eq!(seed_of_sign(true, false), seed_of_sign(false, true));
    assert_eq!(seed_of_sign(true, true), seed_of_sign(false, true));
    assert_ne!(seed_of_sign(false, false), seed_of_sign(true, false));
    assert_eq!(alpha_sign(0.0f32), (false, true));
    assert_eq!(alpha_sign(-0.0f32), (true, true));
    assert_eq!(alpha_sign(1.0f32), (false, false));
    assert_eq!(alpha_sign(-1.0f32), (true, false));
    let f = feasibility_input(1.0f32, 0.5, 1.0);
    assert!(!f.flat && !f.negative && f.moving);
    assert!(feasibility_input(0.0f32, 1.0, 1.0).flat && feasibility_input(1.0f32, 0.0, 1.0).flat);
    assert!(feasibility_input(-1.0f32, 1.0, 1.0).negative);
    assert!(!feasibility_input(-1.0f32, -1.0, 1.0).negative);
    assert!(!feasibility_input(1.0f32, 1.0, 0.0).moving);
    let tiny = 1.0f32.next_down();
    assert!(
        !feasibility_short(&feasibility_input(1.0f32, 0.5, 1.0)),
        "2·k = l² is feasible"
    );
    assert!(feasibility_short(&feasibility_input(
        1.0f32,
        0.5 * tiny,
        1.0
    )));
    assert!(feasibility_short(&feasibility_input(1.0f32, 0.0, 1.0)));
    assert!(!feasibility_short(&feasibility_input(1.0f32, 0.0, 0.0)));
    assert!(feasibility_short(&feasibility_input(1.0f32, -1.0, 0.0)));
    assert!(feasibility_short(&feasibility_input(1.0f32, 1e-6, 1e6)));
    assert!(!feasibility_short(&feasibility_input(1.0f32, 1e6, 1e-6)));
}

#[test]
fn branch_read_seeds_and_feasibility_at_their_boundary() {
    check_seeds();
}

negative_control!(
    branch_read_seeds_and_feasibility_at_their_boundary,
    "the boundary taken as short",
    expected = "2·k = l² read short",
    assert!(
        feasibility_short(&feasibility_input(1.0f32, 0.5, 1.0)),
        "2·k = l² read short"
    )
);

fn check_wide(x: f32, moved: f32) {
    let (mass, _) = wide_state([x, x, x], [[0.0; 2]; 3]);
    let [m, e] = mass[0].words();
    let value = f32::from_bits(m) * 2f32.powi(e as i32);
    assert_eq!(value, x * moved, "a Wide's words are not its value");
}

#[test]
fn branch_read_a_wide_reads_as_its_value() {
    for x in [0.75, 3.0, 1e-5, -2.5] {
        check_wide(x, 1.0);
    }
}

negative_control!(
    branch_read_a_wide_reads_as_its_value,
    "another value",
    expected = "a Wide's words are not its value",
    check_wide(0.75, 2.0)
);
