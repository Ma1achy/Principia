//! The golden harness's scene renderer, which `cargo xtask golden` runs for a case of the harness kind: xtask reaches
//! `validation` only as a dev-dependency (systems_architecture §7.1; R-176, R-187), so it runs this binary, as it runs
//! `gate` (TASK-M1-09; RQ-229).
//!
//! `golden_harness --scene <name>`: renders the synthetic scene `name` (`validation::golden_scene`) on the backend
//! `PRIN_GPU_BACKEND` names (R-206) into an `Rgba32Float` target and writes the float image (`golden_scene::MAGIC`'s
//! format) to stdout. `golden_harness --list` prints the scenes, `m1-numeric`'s then `debug-views`'s.

use std::io::Write as _;
use std::process::ExitCode;

use validation::golden_scene;

fn run(args: &[&str]) -> Result<(), String> {
    match args {
        ["--list"] => {
            for name in golden_scene::NAMES {
                println!("{name}");
            }
            for case in golden_scene::debug_cases()? {
                println!("{}", case.name);
            }
            Ok(())
        }
        ["--scene", name] => {
            let scene = golden_scene::scene(name)?;
            let gpu = validation::gpu::GpuHarness::new().map_err(|e| e.to_string())?;
            let pixels = scene.render(gpu.device(), gpu.queue())?;
            let (width, height) = scene.size();
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
