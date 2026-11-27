//! The parameter types beyond numbers and short vectors: a gradient, a list, a lookup table, a field and an image. A
//! node declares one with `// @uniform <name>: <type> = <default>` (grammar version 2), and its value is a flat list of
//! numbers like any other, so a stain's params, its canonical form and a saved stain hold it unchanged.
//!
//! | type | value | in the uniform block |
//! |---|---|---|
//! | `gradient<N>` | `[space, cyclic, (x, y, z, position) × count]`, `2 ≤ count ≤ N`; `space` 0 is linear RGB, 1 OKLab | `GradientN` |
//! | `list<T, N>` (`T` is `f32` or `vec2/3/4<f32>`) | `count × width` numbers, `count ≤ N` | `ListWxN` |
//! | `lut<N>` | `N` numbers, sampled over `[0, 1]` | `LutN` |
//! | `field` | the index of a read-side field in [`super::source_fields`] | nothing: read as the field, in the code |
//! | `image` | an image's handle, a whole number | a `u32` |
//!
//! A gradient, list and lut are WGSL structs the assembler declares once per type, with the function that reads one
//! (`gradientN_at`, `lutN_at`). Their elements are `vec4<f32>`, 16 bytes, because a uniform array's stride is 16: a
//! gradient stop is `(x, y, z, position)`, a list item is padded to four lanes, a lut packs four values to an element.

use std::fmt::Write as _;

use ledger::constants::RAMP_OKLAB_AB_MAX;

use super::UniformType;

// The plan is unclear here; chose these caps (16 stops, 32 list items, 256 lut entries, 16 KiB a block) because the
// largest block they allow, 16 KiB, is a quarter of the 64 KiB binding limit every backend gives, so a stain of several
// nodes still binds; a node that needs more is refused at its line, naming the cap.
/// The most stops a gradient declares, items a list and entries a lut.
pub const GRADIENT_MAX: u8 = 16;
pub const LIST_MAX: u8 = 32;
pub const LUT_MAX: u16 = 256;
/// The largest uniform block a node declares, in bytes: a quarter of the 64 KiB binding limit every backend gives, so a
/// stain of several nodes with large parameters still binds.
pub const BLOCK_MAX: u64 = 16 * 1024;

// The plan is unclear here; chose a gradient value `[space, cyclic, stops…]` holding both linear RGB and OKLab, because
// today's field ramps mix in linear RGB and the custom-stop ramp in OKLab, and one type must draw both to the same bits.
/// The type `s` names where it is one of the new types (`gradient<8>`, `list<vec3<f32>, 8>`, `lut<64>`, `field`,
/// `image`), `Err` with why where it names one with a bad size, `Ok(None)` where it is not one of them.
pub fn parse_type(s: &str) -> Result<Option<UniformType>, String> {
    let s: String = s.split_whitespace().collect::<Vec<_>>().join("");
    let within = |inner: &str, lo: u32, hi: u32, what: &str| -> Result<u32, String> {
        inner
            .parse::<u32>()
            .ok()
            .filter(|n| (lo..=hi).contains(n))
            .ok_or_else(|| format!("{what} is a whole number from {lo} to {hi}"))
    };
    if s == "field" {
        return Ok(Some(UniformType::Field));
    }
    if s == "image" {
        return Ok(Some(UniformType::Image));
    }
    let Some((head, rest)) = s.split_once('<') else {
        return Ok(None);
    };
    let Some(inner) = rest.strip_suffix('>') else {
        return Ok(None);
    };
    Ok(Some(match head {
        "gradient" => {
            UniformType::Gradient(
                within(inner, 2, GRADIENT_MAX.into(), "a gradient's most stops")? as u8,
            )
        }
        "lut" => UniformType::Lut(within(inner, 2, LUT_MAX.into(), "a lut's length")? as u16),
        "list" => {
            let (item, n) = inner
                .rsplit_once(',')
                .ok_or("a list is `list<item, most>`")?;
            let width = match item {
                "f32" => 1,
                "vec2<f32>" => 2,
                "vec3<f32>" => 3,
                "vec4<f32>" => 4,
                _ => return Err("a list's item is f32 or vec2/3/4<f32>".into()),
            };
            UniformType::List {
                width,
                max: within(n, 1, LIST_MAX.into(), "a list's most items")? as u8,
            }
        }
        _ => return Ok(None),
    }))
}

/// The size of `ty` in a uniform block: its alignment and size in bytes.
pub fn align_size(ty: UniformType) -> (u64, u64) {
    #[cfg(feature = "controls")]
    if probe::narrow_stride() {
        // A control's packer that strides a uniform array by the element's 12 or 16 bytes' packed size, not WGSL's 16.
        if let UniformType::Gradient(n) | UniformType::List { max: n, .. } = ty {
            return (16, 16 + 12 * u64::from(n));
        }
    }
    match ty {
        UniformType::Gradient(n) | UniformType::List { max: n, .. } => (16, 16 + 16 * u64::from(n)),
        UniformType::Lut(n) => (16, 16 * u64::from(n).div_ceil(4)),
        _ => (4, 4),
    }
}

/// A test's control's reach into the packer, on this thread only.
#[cfg(feature = "controls")]
pub mod probe {
    use std::cell::Cell;

    thread_local! {
        static NARROW: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn narrow_stride() -> bool {
        NARROW.with(Cell::get)
    }

    /// `f` run with a gradient's and a list's size counted as if its array's stride were 12, not WGSL's 16.
    pub fn narrow_stride_in<T>(f: impl FnOnce() -> T) -> T {
        let before = NARROW.with(|c| c.replace(true));
        let out = f();
        NARROW.with(|c| c.set(before));
        out
    }
}

/// The WGSL name of the struct of `ty`, for the types that have one.
pub fn struct_name(ty: UniformType) -> Option<String> {
    match ty {
        UniformType::Gradient(n) => Some(format!("Gradient{n}")),
        UniformType::List { width, max } => Some(format!("List{width}x{max}")),
        UniformType::Lut(n) => Some(format!("Lut{n}")),
        _ => None,
    }
}

/// The declarations of the types in `types`, once each, in a fixed order: each struct, with its reader.
pub fn library(types: &[UniformType]) -> String {
    let mut seen: Vec<String> = Vec::new();
    let mut out = String::new();
    for &ty in types {
        let Some(name) = struct_name(ty) else {
            continue;
        };
        if seen.contains(&name) {
            continue;
        }
        seen.push(name.clone());
        let _ = match ty {
            UniformType::Gradient(n) => write!(
                out,
                "\n// A gradient's stops, `(x, y, z, position)`, the first `count` of them live.\n\
                 struct {name} {{\n    count: u32,\n    cyclic: u32,\n    space: u32,\n    reserved: u32,\n    stops: array<vec4<f32>, {n}>,\n}}\n\
                 {}",
                gradient_fn(&name, n)
            ),
            UniformType::List { max, .. } => write!(
                out,
                "\n// A list's first `count` items, each in the first lanes of its `vec4<f32>`.\n\
                 struct {name} {{\n    count: u32,\n    reserved0: u32,\n    reserved1: u32,\n    reserved2: u32,\n    items: array<vec4<f32>, {max}>,\n}}\n"
            ),
            UniformType::Lut(n) => {
                let (elements, last, prev) = (u32::from(n).div_ceil(4), n - 1, n - 2);
                write!(
                    out,
                    "\n// A lookup table's {n} values, four to an element.\n\
                     struct {name} {{\n    values: array<vec4<f32>, {elements}>,\n}}\n\
                     fn lut{n}_at(l: {name}, x: f32) -> f32 {{\n\
                     \x20   let p = clamp(x, 0.0, 1.0) * {last}.0;\n\
                     \x20   let k = min(u32(p), {prev}u);\n\
                     \x20   let u = p - f32(k);\n\
                     \x20   let a = l.values[k / 4u][k % 4u];\n\
                     \x20   let b = l.values[(k + 1u) / 4u][(k + 1u) % 4u];\n\
                     \x20   return mix(a, b, u);\n}}\n"
                )
            }
            _ => Ok(()),
        };
    }
    out
}

/// A gradient's reader, mirroring [`gradient_at`] step for step: the stop at or before `t`, the mix to the next, the
/// ends clamped or, cyclic, wrapped, and OKLab stops converted to linear RGB last. The search for the stop is unrolled,
/// not a loop: a call with a loop counts as an effect to the proof of what matters, which would keep a gradient that
/// only a dead call reads.
fn gradient_fn(name: &str, max: u8) -> String {
    let n = max;
    let mut out = format!("fn gradient{n}_at(g: {name}, x: f32) -> vec3<f32> {{\n");
    for line in [
        "var t = x;",
        "if (g.cyclic != 0u) {",
        "    let f = t - floor(t);",
        "    t = select(f, 0.0, f >= 1.0);",
        "} else {",
        "    t = clamp(t, 0.0, 1.0);",
        "}",
        "let first = g.stops[0];",
        "let last = g.stops[g.count - 1u];",
        "var c = first.xyz;",
        "if (t < first.w || t >= last.w) {",
        "    if (g.cyclic == 0u) {",
        "        // The end is the last segment at its end, not a copy of the last colour: `mix` is the backend's, and",
        "        // a ramp that ends through `mix(a, b, 1.0)` must draw the same bits as a gradient does.",
        "        let p = g.stops[g.count - 2u];",
        "        c = select(mix(p.xyz, last.xyz, 1.0), first.xyz, t < first.w);",
        "    } else {",
        "        let span = (1.0 - last.w) + first.w;",
        "        let offset = select(t - last.w, (1.0 - last.w) + t, t < first.w);",
        "        c = mix(last.xyz, first.xyz, offset / span);",
        "    }",
        "} else {",
        "    var k = 0u;",
    ] {
        out.push_str(&format!("    {line}\n"));
    }
    for j in 1..max {
        out.push_str(&format!(
            "        k = select(k, {j}u, {j}u < g.count && g.stops[{j}].w <= t);\n"
        ));
    }
    for line in [
        "    let a = g.stops[k];",
        "    let b = g.stops[k + 1u];",
        "    c = mix(a.xyz, b.xyz, (t - a.w) / (b.w - a.w));",
        "}",
        "return select(c, oklab_to_linear(c), g.space != 0u);",
    ] {
        out.push_str(&format!("    {line}\n"));
    }
    out.push_str("}\n");
    out
}

// ── Values ────────────────────────────────────────────────────────────────────────────────────────────────────────

fn finite32(v: f64) -> bool {
    (v as f32).is_finite()
}

/// Whether `v` is a value of `ty`, one of the new types; `None` where `ty` is not one.
pub fn admits(ty: UniformType, v: &[f64]) -> Option<bool> {
    Some(match ty {
        UniformType::Gradient(n) => gradient_admits(n, v),
        UniformType::List { width, max } => {
            v.len().is_multiple_of(usize::from(width))
                && v.len() / usize::from(width) <= usize::from(max)
                && v.iter().all(|&x| finite32(x))
        }
        UniformType::Lut(n) => v.len() == usize::from(n) && v.iter().all(|&x| finite32(x)),
        UniformType::Field => {
            v.len() == 1
                && v[0].fract() == 0.0
                && v[0] >= 0.0
                && super::source_fields().is_ok_and(|f| (v[0] as usize) < f.len())
        }
        UniformType::Image => {
            v.len() == 1 && v[0].fract() == 0.0 && (0.0..=4_294_967_295.0).contains(&v[0])
        }
        _ => return None,
    })
}

fn gradient_admits(n: u8, v: &[f64]) -> bool {
    let [space, cyclic, stops @ ..] = v else {
        return false;
    };
    let count = stops.len() / 4;
    if !(*space == 0.0 || *space == 1.0)
        || !(*cyclic == 0.0 || *cyclic == 1.0)
        || stops.len() % 4 != 0
        || !(2..=usize::from(n)).contains(&count)
    {
        return false;
    }
    let ab = RAMP_OKLAB_AB_MAX.number();
    let mut before = 0.0;
    stops.chunks_exact(4).all(|s| {
        let [x, y, z, p] = [s[0], s[1], s[2], s[3]];
        let colour = if *space == 1.0 {
            (0.0..=1.0).contains(&x) && (-ab..=ab).contains(&y) && (-ab..=ab).contains(&z)
        } else {
            finite32(x) && finite32(y) && finite32(z)
        };
        let ok = colour && (0.0..=1.0).contains(&p) && p >= before;
        before = p;
        ok
    })
}

/// Why `v` is not a value of `ty`, in words, for the line that declares the default.
pub fn why_not(ty: UniformType, v: &[f64]) -> String {
    match ty {
        UniformType::Gradient(n) => format!(
            "a gradient is two to {n} stops, positions in [0, 1] never decreasing, each colour finite (OKLab: L in [0, 1], a and b within the register's bound)"
        ),
        UniformType::List { max, .. } => {
            format!("a list holds at most {max} items, each of the declared width, every number finite")
        }
        UniformType::Lut(n) => format!("a lut is {n} finite numbers; this has {}", v.len()),
        UniformType::Field => "a field is the name of a read-side field a source reads".into(),
        UniformType::Image => "an image is a whole-number handle".into(),
        _ => "not a value of the type".into(),
    }
}

// ── Defaults ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// The default of a new type, from the text after `=`: a gradient's `[linear | oklab] [cyclic] <position>:(a, b, c) …`,
/// a list's and a lut's `[<item>, …]` with a vector item in parentheses, a field's name, an image's `none`.
pub fn parse_default(ty: UniformType, text: &str) -> Result<Vec<f64>, String> {
    let text = text.trim();
    match ty {
        UniformType::Gradient(_) => parse_gradient(text),
        UniformType::List { width, .. } => parse_items(text, usize::from(width)),
        UniformType::Lut(_) => parse_items(text, 1),
        UniformType::Field => super::source_fields()
            .map_err(|e| e.to_string())?
            .iter()
            .position(|f| f == text)
            .map(|k| vec![k as f64])
            .ok_or_else(|| format!("`{text}` is no field a source reads")),
        UniformType::Image if text == "none" => Ok(vec![0.0]),
        UniformType::Image => Err("an image's default is `none`".into()),
        _ => Err("not a type with a written default".into()),
    }
}

/// The default text of the gradient `value`, as [`parse_default`] reads it.
pub fn gradient_text(value: &[f64]) -> String {
    let [space, cyclic, stops @ ..] = value else {
        return String::new();
    };
    let mut out = String::from(if *space == 0.0 { "linear" } else { "oklab" });
    if *cyclic != 0.0 {
        out.push_str(" cyclic");
    }
    for s in stops.chunks_exact(4) {
        let _ = write!(
            out,
            " {}:({}, {}, {})",
            ledger::gen::numeric::float(s[3]),
            ledger::gen::numeric::float(s[0]),
            ledger::gen::numeric::float(s[1]),
            ledger::gen::numeric::float(s[2])
        );
    }
    out
}

fn parse_gradient(text: &str) -> Result<Vec<f64>, String> {
    let mut space = 0.0;
    let mut cyclic = 0.0;
    let mut rest = text;
    loop {
        let word = rest.split_whitespace().next().unwrap_or("");
        match word {
            "linear" => space = 0.0,
            "oklab" => space = 1.0,
            "cyclic" => cyclic = 1.0,
            _ => break,
        }
        rest = rest.trim_start()[word.len()..].trim_start();
    }
    let mut out = vec![space, cyclic];
    let mut rest = rest.trim();
    while !rest.is_empty() {
        let (position, tail) = rest
            .split_once(':')
            .ok_or("a stop is `<position>:(a, b, c)`")?;
        let tail = tail.trim_start();
        let (colour, tail) = tail
            .strip_prefix('(')
            .and_then(|t| t.split_once(')'))
            .ok_or("a stop's colour is `(a, b, c)`")?;
        let colour = super::numbers(colour)?;
        if colour.len() != 3 {
            return Err("a stop's colour is three numbers".into());
        }
        let p = super::number(position)?;
        out.extend([colour[0], colour[1], colour[2], p]);
        rest = tail.trim_start();
    }
    Ok(out)
}

fn parse_items(text: &str, width: usize) -> Result<Vec<f64>, String> {
    let inner = text
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .ok_or("a default is `[<item>, …]`")?
        .trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut rest = inner;
    loop {
        if width == 1 {
            let (item, tail) = rest.split_once(',').map_or((rest, ""), |(a, b)| (a, b));
            out.push(super::number(item)?);
            rest = tail;
        } else {
            let t = rest
                .trim_start()
                .strip_prefix('(')
                .ok_or("an item is `(a, b, …)`")?;
            let (item, tail) = t.split_once(')').ok_or("an unclosed `(`")?;
            let v = super::numbers(item)?;
            if v.len() != width {
                return Err(format!("an item is {width} numbers"));
            }
            out.extend(v);
            rest = tail.trim_start().strip_prefix(',').unwrap_or("");
        }
        if rest.trim().is_empty() {
            return Ok(out);
        }
    }
}

// ── Bytes ─────────────────────────────────────────────────────────────────────────────────────────────────────────

// The plan is unclear here; chose an `image` that is a `u32` handle in the block, because binding a texture is the
// pipeline's (a bind group layout entry and an upload), not the declaration's; the handle is what the stain keeps.
/// `value`, a value of `ty` (a new type), as the block stores it: the struct WGSL lays out, padded to its declared
/// size with zeros.
pub fn encode(ty: UniformType, value: &[f64]) -> Option<Vec<u8>> {
    let f = |v: f64| (v as f32).to_le_bytes();
    let mut out: Vec<u8> = Vec::new();
    match ty {
        UniformType::Gradient(_) => {
            let (head, stops) = value.split_at(2.min(value.len()));
            out.extend(((stops.len() / 4) as u32).to_le_bytes());
            out.extend((head.get(1).copied().unwrap_or(0.0) as u32).to_le_bytes());
            out.extend((head.first().copied().unwrap_or(0.0) as u32).to_le_bytes());
            out.extend(0u32.to_le_bytes());
            for s in stops.chunks_exact(4) {
                out.extend(s.iter().flat_map(|&v| f(v)));
            }
        }
        UniformType::List { width, .. } => {
            let width = usize::from(width);
            out.extend(((value.len() / width) as u32).to_le_bytes());
            out.extend([0u8; 12]);
            for item in value.chunks_exact(width) {
                out.extend(item.iter().flat_map(|&v| f(v)));
                out.extend(std::iter::repeat_n(0u8, 4 * (4 - width)));
            }
        }
        UniformType::Lut(_) => out.extend(value.iter().flat_map(|&v| f(v))),
        _ => return None,
    }
    out.resize(align_size(ty).1 as usize, 0);
    Some(out)
}

/// The value `bytes` holds as `ty`, an exact inverse of [`encode`] up to `f32` rounding and the padding: a gradient's
/// and a list's live part, a lut whole.
pub fn decode(ty: UniformType, bytes: &[u8]) -> Option<Vec<f64>> {
    let word = |k: usize| {
        bytes
            .get(4 * k..4 * k + 4)
            .map(|b| [b[0], b[1], b[2], b[3]])
    };
    let u = |k: usize| word(k).map(u32::from_le_bytes);
    let f = |k: usize| word(k).map(|b| f64::from(f32::from_le_bytes(b)));
    match ty {
        UniformType::Gradient(_) => {
            let count = u(0)? as usize;
            let mut out = vec![f64::from(u(2)?), f64::from(u(1)?)];
            for k in 0..4 * count {
                out.push(f(4 + k)?);
            }
            Some(out)
        }
        UniformType::List { width, .. } => {
            let count = u(0)? as usize;
            let mut out = Vec::new();
            for i in 0..count {
                for lane in 0..usize::from(width) {
                    out.push(f(4 + 4 * i + lane)?);
                }
            }
            Some(out)
        }
        UniformType::Lut(n) => (0..usize::from(n)).map(f).collect(),
        _ => None,
    }
}

// ── The CPU twin of the gradient's reader ─────────────────────────────────────────────────────────────────────────

/// WGSL's `mix` for `f32` as one backend computes it: `a (1 − u) + b u`, exact at both ends. Another computes
/// `a + (b − a) u`, which is not: [`gradient_at_with`] takes either.
pub fn mix(a: f32, b: f32, u: f32) -> f32 {
    a * (1.0 - u) + b * u
}

/// The gradient `value` at `t`, linear RGB, in `f32` arithmetic as `gradientN_at` computes it, an OKLab gradient
/// converted last; `None` where `value` is no gradient.
pub fn gradient_at(value: &[f64], t: f32) -> Option<[f32; 3]> {
    gradient_at_with(value, t, &mix)
}

/// [`gradient_at`] with `mix` as a backend computes it.
pub fn gradient_at_with(
    value: &[f64],
    t: f32,
    mix: &dyn Fn(f32, f32, f32) -> f32,
) -> Option<[f32; 3]> {
    let [space, cyclic, stops @ ..] = value else {
        return None;
    };
    let stops: Vec<[f32; 4]> = stops
        .chunks_exact(4)
        .map(|s| [s[0] as f32, s[1] as f32, s[2] as f32, s[3] as f32])
        .collect();
    let (first, last) = (*stops.first()?, *stops.last()?);
    let cyclic = *cyclic != 0.0;
    let t = if cyclic {
        let f = t - t.floor();
        if f >= 1.0 {
            0.0
        } else {
            f
        }
    } else {
        t.clamp(0.0, 1.0)
    };
    let rgb = |s: [f32; 4]| [s[0], s[1], s[2]];
    let lerp = |a: [f32; 4], b: [f32; 4], u: f32| {
        let (a, b) = (rgb(a), rgb(b));
        [0, 1, 2].map(|i| mix(a[i], b[i], u))
    };
    let c = if t < first[3] || t >= last[3] {
        if !cyclic {
            if t < first[3] {
                rgb(first)
            } else {
                lerp(stops[stops.len() - 2], last, 1.0)
            }
        } else {
            let span = (1.0 - last[3]) + first[3];
            let offset = if t < first[3] {
                (1.0 - last[3]) + t
            } else {
                t - last[3]
            };
            lerp(last, first, offset / span)
        }
    } else {
        let k = stops.iter().rposition(|s| s[3] <= t)?;
        let (a, b) = (stops[k], stops[k + 1]);
        lerp(a, b, (t - a[3]) / (b[3] - a[3]))
    };
    Some(if *space != 0.0 {
        render_oklab_to_linear(c)
    } else {
        c
    })
}

fn render_oklab_to_linear(c: [f32; 3]) -> [f32; 3] {
    crate::colour::space::oklab_to_linear(c)
}

/// The lut `value` at `x` in `[0, 1]`, as `lutN_at` computes it.
pub fn lut_at(value: &[f64], x: f32) -> Option<f32> {
    let n = value.len();
    if n < 2 {
        return None;
    }
    let p = x.clamp(0.0, 1.0) * (n - 1) as f32;
    let k = (p as usize).min(n - 2);
    let u = p - k as f32;
    Some(mix(value[k] as f32, value[k + 1] as f32, u))
}

// ── A field, read in the code ─────────────────────────────────────────────────────────────────────────────────────

/// The expression that reads the read-side field `field` as the value of a source's `Field`: a `bool` as 0 or 1, a
/// `vec3<f32>` as it is, anything else converted to `f32`, as a field's own source reads it. It names the node
/// function's `ctx`.
pub fn field_read(field: &str) -> Result<String, super::AssembleError> {
    Ok(match super::field_type(field)? {
        "vec3<f32>" => format!("(ctx.sample.{field})"),
        "bool" => format!("select(0.0, 1.0, ctx.sample.{field})"),
        _ => format!("f32(ctx.sample.{field})"),
    })
}

/// `field` as a source's `Field` value, as `as_field(uniforms.<name>)` is written into the code.
fn as_field(field: &str) -> Result<String, super::AssembleError> {
    Ok(match super::field_type(field)? {
        "vec3<f32>" => format!("Field(ctx.sample.{field}, 0.0)"),
        "bool" => format!("Field(select(0.0, 1.0, ctx.sample.{field}), 0.0, 0.0, 0.0)"),
        _ => format!("Field(f32(ctx.sample.{field}), 0.0, 0.0, 0.0)"),
    })
}

/// Whether the fields at indices `a` and `b` of [`super::source_fields`] are both vectors or both scalars.
pub fn same_field_kind(a: f64, b: f64) -> bool {
    let Ok(names) = super::source_fields() else {
        return false;
    };
    let vector = |i: f64| {
        names
            .get(i as usize)
            .is_some_and(|f| super::field_type(f).is_ok_and(|t| t == "vec3<f32>"))
    };
    vector(a) == vector(b)
}

/// Whether `text` reads `uniforms.<name>` anywhere, or reads a word (`uniforms.<param>.<word>`) that reads it.
pub fn reads(text: &str, name: &str) -> bool {
    use crate::lex::Tok;
    let Ok(toks) = super::lex(text) else {
        return true;
    };
    let sig: Vec<&str> = toks
        .iter()
        .filter(|t| !matches!(t.0, Tok::Comment | Tok::Space))
        .map(|&(_, a, b)| &text[a..b])
        .collect();
    sig.windows(3)
        .any(|w| w[0] == "uniforms" && w[1] == "." && w[2] == name)
        || sig.windows(5).any(|w| {
            w[0] == "uniforms"
                && w[1] == "."
                && w[3] == "."
                && crate::colour::field_ramp::uses(w[2], w[4]).contains(&name)
        })
}

/// `text` with each baked uniform in `baked` written into it as `chosen(uniform)` gives it ([`super::Uniform::is_baked`]):
/// a `field` read as the field, and any other read through a word, `uniforms.<name>.<word>`, which a ramp's
/// [`crate::colour::field_ramp::word`] writes as code; and the line after each `//? when uniforms.<name>.<word>` kept
/// only where the word has a value. It is how a parameter reaches the shader: the code reads the field, the curve and
/// the polarity, so the picture's proof, its field set and its code key all see them, and choosing another is a
/// different code.
// The plan is unclear here; chose to write the field into the code, because a stored field is read by name, and a
// runtime index would make every field read and the proof of what a stain needs useless. A curve, a polarity and a
// range are written in for the same reason: a literal is what the picture was drawn with.
pub fn specialise(
    text: &str,
    baked: &[&super::Uniform],
    chosen: &dyn Fn(&super::Uniform) -> Vec<f64>,
) -> Result<String, super::AssembleError> {
    use crate::lex::Tok;
    if baked.is_empty() {
        return Ok(text.to_owned());
    }
    let value = |name: &str| baked.iter().find(|u| u.name == name).map(|u| chosen(u));
    let say = |e: String| super::AssembleError::Occupant(e);
    // A line that a `//? when` says is not there, and the line itself.
    let mut kept = String::with_capacity(text.len());
    let mut hide = false;
    for line in text.split_inclusive('\n') {
        if let Some(cond) = line.trim().strip_prefix("//? when ") {
            let word = cond
                .trim()
                .strip_prefix("uniforms.")
                .and_then(|c| c.split_once('.'))
                .ok_or_else(|| say(format!("`//? when {}` is `uniforms.<name>.<word>`", cond.trim())))?;
            hide = crate::colour::field_ramp::word(&value, word.0, word.1)
                .map_err(&say)?
                .is_none();
            continue;
        }
        if !std::mem::take(&mut hide) {
            kept.push_str(line);
        }
    }
    let text = kept.as_str();
    let toks = super::lex(text)?;
    let sig: Vec<usize> = (0..toks.len())
        .filter(|&k| !matches!(toks[k].0, Tok::Comment | Tok::Space))
        .collect();
    let at = |k: usize| sig.get(k).map_or("", |&t| &text[toks[t].1..toks[t].2]);
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    for k in 0..sig.len() {
        if at(k) != "uniforms" || toks[sig[k]].0 != Tok::Ident || (k > 0 && at(k - 1) == ".") {
            continue;
        }
        let Some(u) = baked.iter().find(|u| u.name == at(k + 2)) else {
            continue;
        };
        if at(k + 1) != "." {
            continue;
        }
        let is_word = at(k + 3) == "."
            && crate::colour::field_ramp::knows(&u.name, at(k + 4));
        if is_word {
            let written = crate::colour::field_ramp::word(&value, &u.name, at(k + 4))
                .map_err(&say)?
                .ok_or_else(|| say(format!("`uniforms.{}.{}` has no value here", u.name, at(k + 4))))?;
            out.push_str(&text[from..toks[sig[k]].1]);
            out.push_str(&written);
            from = toks[sig[k + 4]].2;
            continue;
        }
        if u.ty != super::UniformType::Field {
            return Err(say(format!(
                "`{}` is baked into the code: read it as `uniforms.{}.<word>`",
                u.name, u.name
            )));
        }
        let names = u.field_names()?;
        let Some(name) = chosen(u).first().and_then(|&i| names.get(i as usize)) else {
            return Err(say(format!("`{}` names no field a source reads", u.name)));
        };
        // `as_field(uniforms.<name>)` is a source's whole `Field` value: the vector in `.xyz`, a scalar in `.x`.
        if k >= 2 && at(k - 1) == "(" && at(k - 2) == "as_field" && at(k + 3) == ")" {
            out.push_str(&text[from..toks[sig[k - 2]].1]);
            out.push_str(&as_field(name)?);
            from = toks[sig[k + 3]].2;
            continue;
        }
        out.push_str(&text[from..toks[sig[k]].1]);
        out.push_str(&field_read(name)?);
        from = toks[sig[k + 2]].2;
    }
    out.push_str(&text[from..]);
    Ok(out)
}
