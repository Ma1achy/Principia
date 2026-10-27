//! The golden harness's scene renderer, which `cargo xtask golden` runs for a case of the harness kind: xtask reaches
//! `validation` only as a dev-dependency (systems_architecture §7.1; R-176, R-187), so it runs this binary, as it runs
//! `gate` (TASK-M1-09; RQ-229).
//!
//! `golden_harness --scene <name>`: renders the synthetic scene `name` (`validation::golden_scene`, a structural
//! scene, `validation::structural_scene`, TASK-M1-13, a shape-sphere scene, `validation::shape_scene`, TASK-M2-28, or
//! a DECODE preset scene, `validation::decode_preset::golden`, TASK-M2-25) on the backend `PRIN_GPU_BACKEND` names
//! (R-206) into an `Rgba32Float` target and writes the float image (`golden_scene::MAGIC`'s format) to stdout.
//! `golden_harness --list` prints the scenes, `m1-numeric`'s, then `debug-views`'s, then the structural ones, then the
//! shape sphere's, then the DECODE preset's.

use std::io::Write as _;
use std::process::ExitCode;

use validation::{decode_preset, golden_scene, shape_scene, structural_scene};

/// A scene of either kind.
enum Scene {
    Field(golden_scene::Scene),
    Structural(structural_scene::StructuralScene),
    Decode(decode_preset::Scene),
}

fn run(args: &[&str]) -> Result<(), String> {
    match args {
        ["--list"] => {
            for name in golden_scene::NAMES {
                println!("{name}");
            }
            for case in golden_scene::debug_cases()? {
                println!("{}", case.name);
            }
            for name in structural_scene::NAMES {
                println!("{name}");
            }
            for name in shape_scene::NAMES {
                println!("{name}");
            }
            for chart in render::presets::DecodeChart::ALL {
                println!("{}", decode_preset::golden_name(chart));
            }
            Ok(())
        }
        ["--scene", name] => {
            // The scene is looked up before the device is opened, so an unknown name fails naming the scenes.
            let scene = if structural_scene::NAMES.contains(name) {
                Scene::Structural(structural_scene::scene(name)?)
            } else if shape_scene::NAMES.contains(name) {
                Scene::Field(shape_scene::scene(name)?.scene)
            } else if let Some(chart) = decode_preset::golden_chart(name) {
                Scene::Decode(decode_preset::golden(chart)?)
            } else {
                Scene::Field(golden_scene::scene(name)?)
            };
            let gpu = validation::gpu::GpuHarness::new().map_err(|e| e.to_string())?;
            let ((width, height), pixels) = match scene {
                Scene::Field(s) => (s.size(), s.render(gpu.device(), gpu.queue())?),
                Scene::Structural(s) => (s.size(), s.render(gpu.device(), gpu.queue())?),
                Scene::Decode(s) => (s.size(), s.render(gpu.device(), gpu.queue())?),
            };
            let mut stdout = std::io::stdout().lock();
            stdout
                .write_all(&golden_scene::encode_image(width, height, &pixels))
                .and_then(|()| stdout.flush())
                .map_err(|e| format!("cannot write the image to stdout: {e}"))
        }
        _ => Err(format!(
            "unrecognised arguments `{}`; usage: golden_harness (--scene <name> | --list)",
            args.join(" ")
        )),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("golden_harness: {message}");
            ExitCode::FAILURE
        }
    }
}
