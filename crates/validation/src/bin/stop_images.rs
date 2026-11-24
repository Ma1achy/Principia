//! `stop_images <patchwork | stop-on-escape | control> [--view <zero | config_stability>] [--n <n>] [--propose]
//! [--out <dir>]`: the stop images on the app's zero view, or on `config_stability` (`validation::stop_images`), run by
//! `cargo xtask golden patchwork` and `cargo xtask golden stop-on-escape`. `control` runs the stop arm under the energy
//! floor, pitfalls §1.3's experiment, and reports its seams, strata and each leg of the tolerance: the seam check must
//! fire on it and the tolerance must fail it. Exits non-zero on any failure (for `control`: where the seam check does
//! not fire or the tolerance holds).

use std::path::PathBuf;
use std::process::ExitCode;

use validation::stop_images::{
    control_seams, difference, distribution, measure, render, run, theta_colour,
    tolerance_failures, write_png, Arm, Experiment, Image, Rule, View, TOLERANCE,
};

fn main() -> ExitCode {
    match main_inner() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("stop_images: {e}");
            ExitCode::from(2)
        }
    }
}

fn main_inner() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut n = 64usize;
    let mut propose = false;
    let mut out: Option<PathBuf> = None;
    let mut which = None;
    let mut view = View::Zero;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--n" => {
                n = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("--n takes a size")?;
            }
            "--propose" => propose = true,
            "--view" => view = View::parse(it.next().ok_or("--view takes a view")?)?,
            "--out" => out = Some(PathBuf::from(it.next().ok_or("--out takes a directory")?)),
            w @ ("patchwork" | "stop-on-escape" | "control") => which = Some(w.to_owned()),
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let which = which.ok_or(
        "usage: stop_images <patchwork | stop-on-escape | control> [--view <zero | config_stability>] [--n <n>] \
         [--propose] [--out <dir>]",
    )?;
    let view = view.spec()?;
    let started = std::time::Instant::now();
    if which == "control" {
        let pixels = render(&view, n, Rule::EnergyFloor)?;
        let (seams, hist) = control_seams(n, &pixels);
        let stopped: Image = pixels.iter().map(|p| theta_colour(&p.stop)).collect();
        let live: Image = pixels.iter().map(|p| theta_colour(&p.live)).collect();
        let (differ, q) = distribution(&difference(&stopped, &live));
        println!(
            "control: per-pixel colour difference (theta) against the live arm: {differ:.4} of pixels differ; p50 {} \
             p90 {} p99 {} p99.9 {} max {}",
            q[0], q[1], q[2], q[3], q[4]
        );
        if std::env::var_os("STOP_IMAGES_PAIRS").is_some() {
            let escaped: Vec<bool> = pixels
                .iter()
                .map(|p| {
                    kernel::payload::sd_state(p.stop.packed_a) == kernel::payload::STATE_ESCAPE
                })
                .collect();
            let t_end: Vec<u32> = pixels
                .iter()
                .map(|p| kernel::payload::tm_t_end_step(p.stop.times))
                .collect();
            for (s, r) in
                validation::stop_images::boundary_steps(n, &stopped, &live, &escaped, &t_end)
            {
                println!("pair {s} {r}");
            }
        }
        if let Some(dir) = &out {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            write_png(&dir.join("theta_stop_energy_floor.png"), n, &stopped, 4)?;
            let d: Image = difference(&stopped, &live)
                .iter()
                .map(|&x| [x; 3])
                .collect();
            write_png(&dir.join("theta_difference_energy_floor.png"), n, &d, 4)?;
        }
        let m = measure(&pixels, Arm::Live);
        let legs = tolerance_failures(&m, TOLERANCE);
        println!(
            "control under the tolerance {TOLERANCE:?}: max |dtheta| {:.4e} rad (<= {}), max colour step {} (<= {}), \
             escapes stop {} live {} (equal): {}",
            m.max_dtheta,
            TOLERANCE.max_dtheta,
            m.max_step,
            TOLERANCE.max_step,
            m.stop_escapes,
            m.live_escapes,
            if legs.is_empty() {
                "within: the tolerance does not fail the control".to_owned()
            } else {
                format!("fails {} of 3 legs, as it must ({})", legs.len(), legs.join("; "))
            }
        );
        println!(
            "control (the stop arm under the energy floor), {}, {n} x {n}: {} seam pairs of {} stratum-boundary pairs: \
             {}; t_end of {} escape-terminated pixels takes {} values ({:.1} s)",
            view.describe(),
            seams.seams,
            seams.boundaries,
            if seams.fires() { "SEAM STRUCTURE (the check fires, as it must)" } else { "none: the check did not fire" },
            hist.iter().map(|(_, c)| c).sum::<usize>(),
            hist.len(),
            started.elapsed().as_secs_f64()
        );
        return Ok(seams.fires() && !legs.is_empty());
    }
    let experiment = if which == "patchwork" {
        Experiment::Patchwork
    } else {
        Experiment::StopOnEscape
    };
    let pixels = render(&view, n, Rule::Default)?;
    let report = run(&view, experiment, n, &pixels, out.as_deref(), propose)?;
    print!("{}", report.text);
    println!("  ({:.1} s)", started.elapsed().as_secs_f64());
    for f in &report.failures {
        println!("  FAIL: {f}");
    }
    Ok(report.failures.is_empty())
}
