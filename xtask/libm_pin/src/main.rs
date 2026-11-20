//! `libm-pin write <file> [<host>]`: this build's arm over every chart and zoom, written to `<file>` as one header line,
//! `libm-pin 1 <arm> <host> <width> <height>`, then each chart's digests at each zoom, row by row, as little-endian u64s
//! (the library's note). `<host>` names the machine, `<os>-<arch>` by default. Rows are shared among the threads the
//! machine has.

use std::io::Write;
use std::thread;

use libm_pin::{arm, row, CHARTS, HEIGHT, WIDTH, ZOOMS};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (file, host) = match args.as_slice() {
        [w, file] if w == "write" => (
            file.clone(),
            format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        ),
        [w, file, host] if w == "write" => (file.clone(), host.clone()),
        _ => {
            eprintln!("usage: libm-pin write <file> [<host>]");
            std::process::exit(2);
        }
    };
    let (w, h) = (WIDTH as usize, HEIGHT as usize);
    let threads = thread::available_parallelism().map_or(1, usize::from);
    let mut out =
        std::fs::File::create(&file).unwrap_or_else(|e| panic!("cannot create {file}: {e}"));
    writeln!(out, "libm-pin 1 {} {host} {WIDTH} {HEIGHT}", arm()).expect("the header");
    for (chart, name) in CHARTS.iter().enumerate() {
        for zoom in ZOOMS {
            let mut grid = vec![0u64; w * h];
            let rows_each = h.div_ceil(threads);
            thread::scope(|s| {
                for (k, part) in grid.chunks_mut(rows_each * w).enumerate() {
                    s.spawn(move || {
                        for (j, slots) in part.chunks_mut(w).enumerate() {
                            row(
                                chart,
                                zoom,
                                (k * rows_each + j) as u32,
                                WIDTH,
                                HEIGHT,
                                slots,
                            );
                        }
                    });
                }
            });
            let bytes: Vec<u8> = grid.iter().flat_map(|d| d.to_le_bytes()).collect();
            out.write_all(&bytes).expect("the digests");
            eprintln!("libm-pin {}: {name} at 2^{zoom} done", arm());
        }
    }
}
