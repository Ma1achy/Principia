//! The float rules' constant-expression evaluator ([`constant_floats`]), value by value (REQ-RENDER-083; R-351,
//! R-352): a comparison operand is an inf or NaN constant, or a finite-max stand-in, if it is "a constant expression
//! that evaluates to" one, so the evaluator must give the value WGSL gives every constant expression naga leaves
//! unfolded: every operator, conversion and `bitcast`, `select`, `all` and `any`, and every built-in WGSL allows in a
//! constant expression, over float, integer and bool scalars, vectors and matrices; integers of every width naga
//! has (u16, i16, u64 and i64 as well as u32 and i32); and a NaN made by a `bitcast`, whose bit pattern (sign and
//! payload) is as known as any other value's, through a `bitcast` back, a component, a swizzle, a `select` or a
//! `transpose`, for f16, f32 and f64.
//!
//! Each case is an expression bound to a `let` in a function of its own; the test reads the `let`'s expression from
//! naga's IR (checking naga left it unfolded, so the evaluator, not naga, gives the value) and compares
//! [`constant_floats`]'s value bit for bit (any NaN matching any NaN). The leaves are `bitcast`s, which naga does not
//! fold: `F(x)` is the f32 `x` (`bitcast<f32>(<x's bits>u)`), `H(x)` and `D(x)` that f32 converted to f16 and f64,
//! `I(n)` the i32 and `U(n)` the u32 `n` (by bitcast), and `B(true)` and `B(false)` comparisons of such floats. An
//! integer or bool result is read through a conversion to f32. Each test has a registered negative control (R-176):
//! the same cases against perturbed values.

use naga::valid::{Capabilities, ValidationFlags, Validator};
use naga::Expression;
use validation::negative_control;
use xtask::lint_wgsl::constant_floats;

/// A case: an expression, and its float values (`None`: not a constant of floats WGSL defines).
type Case = (String, Option<Vec<f64>>);

/// `x` rounded to f32.
fn r32(x: f64) -> f64 {
    f64::from(x as f32)
}

/// `expr` with its leaves (`F(..)`, `H(..)`, `D(..)`, `I(..)`, `U(..)`, `B(..)`) spelled out as WGSL.
fn expand(expr: &str) -> String {
    let mut out = String::new();
    let mut rest = expr;
    while let Some(at) = rest.find('(') {
        let head = &rest[..at];
        let marker = head.chars().last();
        let bare = head
            .chars()
            .rev()
            .nth(1)
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        let close = rest[at..].find(')').map(|c| at + c);
        let leaf = match (marker, bare, close) {
            (Some(m @ ('F' | 'H' | 'D' | 'I' | 'U' | 'B')), true, Some(close)) => {
                let arg = &rest[at + 1..close];
                let wgsl = match m {
                    'F' => format!("bitcast<f32>(0x{:08x}u)", f32_bits(arg)),
                    'H' => format!("f16(bitcast<f32>(0x{:08x}u))", f32_bits(arg)),
                    'D' => format!("f64(bitcast<f32>(0x{:08x}u))", f32_bits(arg)),
                    'I' => format!("bitcast<i32>(0x{:08x}u)", int_bits(arg)),
                    'U' => format!("bitcast<u32>(0x{:08x}u)", int_bits(arg)),
                    _ if arg == "true" => expand("(F(0.0) < F(1.0))"),
                    _ => expand("(F(1.0) < F(0.0))"),
                };
                Some((head.len() - 1, wgsl, close))
            }
            _ => None,
        };
        match leaf {
            Some((start, wgsl, close)) => {
                out.push_str(&rest[..start]);
                out.push_str(&wgsl);
                rest = &rest[close + 1..];
            }
            None => {
                out.push_str(&rest[..=at]);
                rest = &rest[at + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The bits of the f32 `arg` (`NaN` and `inf` included).
fn f32_bits(arg: &str) -> u32 {
    arg.parse::<f32>()
        .unwrap_or_else(|e| panic!("`{arg}` is no f32: {e}"))
        .to_bits()
}

/// The 32 bits of the integer `arg`, decimal (signed) or hex (`0x…`).
fn int_bits(arg: &str) -> u32 {
    let v = match arg.strip_prefix("0x") {
        Some(hex) => i64::from_str_radix(hex, 16),
        None => arg.parse::<i64>(),
    }
    .unwrap_or_else(|e| panic!("`{arg}` is no integer: {e}"));
    v as u32
}

/// The evaluator's float values of `expr`, bound to a `let` naga leaves unfolded.
fn eval(expr: &str) -> Option<Vec<f64>> {
    let source = format!(
        "enable f16;\nenable wgpu_int16;\nfn f() {{\n    let v = {};\n}}\n",
        expand(expr)
    );
    let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| {
        panic!(
            "`{expr}` does not parse: {}\n{source}",
            e.emit_to_string(&source)
        )
    });
    Validator::new(ValidationFlags::all(), Capabilities::all())
        .validate(&module)
        .unwrap_or_else(|e| panic!("`{expr}` does not validate: {e:?}\n{source}"));
    let (_, function) = module.functions.iter().next().expect("the case's function");
    let (&h, _) = function
        .named_expressions
        .iter()
        .find(|(_, name)| name.as_str() == "v")
        .expect("the case's `let`");
    assert!(
        !matches!(function.expressions[h], Expression::Literal(_)),
        "naga folded `{expr}`, so it does not test the evaluator"
    );
    constant_floats(&module, &function.expressions, h)
}

/// Whether `a` and `b` are the same values, bit for bit, any NaN matching any NaN.
fn same(a: &Option<Vec<f64>>, b: &Option<Vec<f64>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(x, y)| x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()))
        }
        (None, None) => true,
        _ => false,
    }
}

/// Panics, naming every case whose value is not its expected one.
fn check(cases: &[Case]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(expr, want)| {
            let got = eval(expr);
            (!same(&got, want)).then(|| format!("`{expr}`: got {got:?}, want {want:?}"))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of {} case(s) mismatched:\n{}",
        wrong.len(),
        cases.len(),
        wrong.join("\n")
    );
}

/// `cases` with every expected value moved: each float plus one, and `None` made a value.
#[cfg(feature = "controls")]
fn perturbed(cases: &[Case]) -> Vec<Case> {
    cases
        .iter()
        .map(|(expr, want)| {
            let moved = match want {
                Some(v) => v.iter().map(|x| x + 1.0).collect(),
                None => vec![0.0],
            };
            (expr.clone(), Some(moved))
        })
        .collect()
}

/// A case of `expr` with the values `want`.
fn case(expr: &str, want: &[f64]) -> Case {
    (expr.to_string(), Some(want.to_vec()))
}

/// The bytes of the u32 expression `p`, low first, as f32s.
fn bytes(p: &str) -> String {
    format!(
        "vec4<f32>(f32(({p}) & 0xffu), f32((({p}) >> 8u) & 0xffu), f32((({p}) >> 16u) & 0xffu), \
         f32(({p}) >> 24u))"
    )
}

/// The 16-bit halves of the u32 expression `p`, low first, as f32s.
fn halves(p: &str) -> String {
    format!("vec2<f32>(f32(({p}) & 0xffffu), f32(({p}) >> 16u))")
}

/// `select(1.0, 2.0, cond)`: 2 if the bool expression `cond` is true, 1 if not.
fn pick(cond: &str) -> String {
    format!("select(F(1.0), F(2.0), {cond})")
}

const A: &str = "mat2x2<f32>(vec2<f32>(F(1.0), F(2.0)), vec2<f32>(F(3.0), F(4.0)))";
const BM: &str = "mat2x2<f32>(vec2<f32>(F(5.0), F(6.0)), vec2<f32>(F(7.0), F(8.0)))";
const V: &str = "vec2<f32>(F(5.0), F(6.0))";

/// Operators, comparisons, `select`, `all` and `any`, matrix products, zero values and conversions.
fn operators() -> Vec<Case> {
    let max = f64::from(u32::MAX);
    let mut cases = vec![
        case("F(6.0) + F(4.0)", &[10.0]),
        case("F(6.0) - F(4.0)", &[2.0]),
        case("F(6.0) * F(4.0)", &[24.0]),
        case("F(6.0) / F(4.0)", &[1.5]),
        case("F(6.0) % F(4.0)", &[2.0]),
        case("-F(2.0)", &[-2.0]),
        case("D(1.5) + D(1.0)", &[2.5]),
        case("f32(U(12) + U(10))", &[22.0]),
        case("f32(U(3) - U(7))", &[r32(4_294_967_292.0)]),
        case("f32(U(12) * U(10))", &[120.0]),
        case("f32(U(0x80000000) * U(2))", &[0.0]),
        case("f32(U(0xffffffff) + U(2))", &[1.0]),
        case("f32(U(12) / U(5))", &[2.0]),
        case("f32(U(12) / U(0))", &[12.0]),
        case("f32(U(12) % U(5))", &[2.0]),
        case("f32(U(12) % U(0))", &[0.0]),
        case("f32(U(12) & U(10))", &[8.0]),
        case("f32(U(12) ^ U(10))", &[6.0]),
        case("f32(U(12) | U(10))", &[14.0]),
        case("f32(U(3) << U(2))", &[12.0]),
        case("f32(U(12) >> U(2))", &[3.0]),
        case("f32(U(3) << U(33))", &[6.0]),
        case("f32(~U(0))", &[r32(max)]),
        case("f32(I(-7) + I(3))", &[-4.0]),
        case("f32(I(2147483647) + I(1))", &[-2_147_483_648.0]),
        case("f32(I(-7) - I(3))", &[-10.0]),
        case("f32(I(-7) * I(3))", &[-21.0]),
        case("f32(I(-7) / I(2))", &[-3.0]),
        case("f32(I(-7) / I(0))", &[-7.0]),
        case("f32(I(-2147483648) / I(-1))", &[-2_147_483_648.0]),
        case("f32(I(-7) % I(2))", &[-1.0]),
        case("f32(I(-7) % I(0))", &[0.0]),
        case("f32(I(-2147483648) % I(-1))", &[0.0]),
        case("f32(I(12) & I(10))", &[8.0]),
        case("f32(I(12) ^ I(10))", &[6.0]),
        case("f32(I(12) | I(10))", &[14.0]),
        case("f32(I(-8) >> U(1))", &[-4.0]),
        case("f32(I(3) << U(2))", &[12.0]),
        case("f32(-I(5))", &[-5.0]),
        case("f32(~I(0))", &[-1.0]),
        case(&pick("B(true) & B(false)"), &[1.0]),
        case(&pick("B(true) | B(false)"), &[2.0]),
        case(&pick("B(true) | B(true)"), &[2.0]),
        case(&pick("B(true) == B(false)"), &[1.0]),
        case(&pick("B(true) != B(false)"), &[2.0]),
        case(&pick("!B(true)"), &[1.0]),
        case(&pick("U(1) < U(2)"), &[2.0]),
        case(&pick("I(-1) < I(2)"), &[2.0]),
        case(
            "select(vec2<f32>(F(1.0), F(2.0)), vec2<f32>(F(3.0), F(4.0)), vec2<bool>(B(true), B(false)))",
            &[3.0, 2.0],
        ),
        case(
            "select(vec2<f32>(F(1.0), F(2.0)), vec2<f32>(F(3.0), F(4.0)), B(true))",
            &[3.0, 4.0],
        ),
        case(&pick("all(vec2<bool>(B(true), B(false)))"), &[1.0]),
        case(&pick("all(vec2<bool>(B(true), B(true)))"), &[2.0]),
        case(&pick("any(vec2<bool>(B(true), B(false)))"), &[2.0]),
        case(&pick("any(vec2<bool>(B(false), B(false)))"), &[1.0]),
        case(&format!("{A} * {BM}"), &[23.0, 34.0, 31.0, 46.0]),
        case(&format!("{A} * {V}"), &[23.0, 34.0]),
        case(&format!("{V} * {A}"), &[17.0, 39.0]),
        case(&format!("{A} + {BM}"), &[6.0, 8.0, 10.0, 12.0]),
        case(&format!("{A} * F(2.0)"), &[2.0, 4.0, 6.0, 8.0]),
        case(&format!("F(2.0) * {A}"), &[2.0, 4.0, 6.0, 8.0]),
        case(
            "mat3x2<f32>(vec2<f32>(F(1.0), F(2.0)), vec2<f32>(F(3.0), F(4.0)), vec2<f32>(F(5.0), F(6.0))) \
             * vec3<f32>(F(1.0))",
            &[9.0, 12.0],
        ),
        case("vec2<f32>() + vec2<f32>(F(1.0), F(2.0))", &[1.0, 2.0]),
        case(&format!("mat2x2<f32>() + {A}"), &[1.0, 2.0, 3.0, 4.0]),
        case("vec2<f32>(vec2<u32>() + vec2<u32>(U(1), U(2)))", &[1.0, 2.0]),
        case("vec2<f32>(vec2<i32>() - vec2<i32>(I(1), I(2)))", &[-1.0, -2.0]),
        case(
            "select(vec2<f32>(F(1.0)), vec2<f32>(F(2.0)), vec2<bool>())",
            &[1.0, 1.0],
        ),
        case("array<f32, 2>()[I(1)] + F(1.0)", &[1.0]),
        case("f32(U(7))", &[7.0]),
        case("f32(I(-7))", &[-7.0]),
        case("f32(B(true))", &[1.0]),
        case("f32(B(false))", &[0.0]),
        case("f16(U(65520))", &[f64::INFINITY]),
        case("bitcast<f32>(F(2.0))", &[2.0]),
        case("bitcast<f32>(F(NaN))", &[f64::NAN]),
        case("f32(u32(F(-3.5)))", &[0.0]),
        case("f32(u32(F(3.7)))", &[3.0]),
        case("f32(u32(F(5e9)))", &[r32(max)]),
        case("f32(i32(F(-3.7)))", &[-3.0]),
        case("f32(u32(B(true)))", &[1.0]),
        case("f32(i32(B(true)))", &[1.0]),
        case("f32(bitcast<u32>(F(1.0)))", &[1_065_353_216.0]),
        case("f32(bitcast<i32>(F(-2.0)))", &[-1_073_741_824.0]),
        case("f32(u32(I(-1)))", &[r32(max)]),
        case("f32(i32(U(0xffffffff)))", &[-1.0]),
        case(&pick("bool(F(0.0))"), &[1.0]),
        case(&pick("bool(F(-0.5))"), &[2.0]),
        case(&pick("bool(U(0))"), &[1.0]),
        case(&pick("bool(U(3))"), &[2.0]),
        case(&pick("bool(I(0))"), &[1.0]),
        case(&pick("bool(I(-3))"), &[2.0]),
        case(&pick("bool(B(true))"), &[2.0]),
        ("U(3)".to_string(), None),
        ("I(3)".to_string(), None),
        ("B(true)".to_string(), None),
    ];
    // Each comparison on the pairs (1, 2), (2, 2) and (2, 1).
    for (op, want) in [
        ("<", [1.0, 0.0, 0.0]),
        ("<=", [1.0, 1.0, 0.0]),
        (">", [0.0, 0.0, 1.0]),
        (">=", [0.0, 1.0, 1.0]),
        ("==", [0.0, 1.0, 0.0]),
        ("!=", [1.0, 0.0, 1.0]),
    ] {
        cases.push(case(
            &format!(
                "select(vec3<f32>(F(0.0)), vec3<f32>(F(1.0)), vec3<f32>(F(1.0), F(2.0), F(2.0)) {op} \
                 vec3<f32>(F(2.0), F(2.0), F(1.0)))"
            ),
            &want,
        ));
    }
    cases
}

/// The float built-ins, component-wise, in f32 (and one in f16).
fn float_builtins() -> Vec<Case> {
    vec![
        case("abs(F(-2.5))", &[2.5]),
        case("min(F(1.0), F(2.0))", &[1.0]),
        case("max(F(1.0), F(2.0))", &[2.0]),
        case("clamp(F(5.0), F(1.0), F(3.0))", &[3.0]),
        case("clamp(F(0.0), F(1.0), F(3.0))", &[1.0]),
        case("saturate(F(2.0))", &[1.0]),
        case("saturate(F(-1.0))", &[0.0]),
        case("cos(F(1.0))", &[r32(1f64.cos())]),
        case("cosh(F(1.0))", &[r32(1f64.cosh())]),
        case("sin(F(1.0))", &[r32(1f64.sin())]),
        case("sinh(F(1.0))", &[r32(1f64.sinh())]),
        case("tan(F(1.0))", &[r32(1f64.tan())]),
        case("tanh(F(1.0))", &[r32(1f64.tanh())]),
        case("acos(F(0.5))", &[r32(0.5f64.acos())]),
        case("asin(F(0.5))", &[r32(0.5f64.asin())]),
        case("atan(F(2.0))", &[r32(2f64.atan())]),
        case("atan2(F(1.0), F(2.0))", &[r32(1f64.atan2(2.0))]),
        case("asinh(F(2.0))", &[r32(2f64.asinh())]),
        case("acosh(F(2.0))", &[r32(2f64.acosh())]),
        case("atanh(F(0.5))", &[r32(0.5f64.atanh())]),
        case("radians(F(180.0))", &[r32(std::f64::consts::PI)]),
        case("degrees(F(1.0))", &[r32(1f64.to_degrees())]),
        case("ceil(F(1.5))", &[2.0]),
        case("floor(F(1.5))", &[1.0]),
        case("round(F(2.5))", &[2.0]),
        case("round(F(3.5))", &[4.0]),
        case("fract(F(-1.25))", &[0.75]),
        case("trunc(F(-1.5))", &[-1.0]),
        case("exp(F(1.0))", &[r32(std::f64::consts::E)]),
        case("exp2(F(3.0))", &[8.0]),
        case("log(F(2.0))", &[r32(std::f64::consts::LN_2)]),
        case("log2(F(8.0))", &[3.0]),
        case("pow(F(2.0), F(3.0))", &[8.0]),
        case("sign(F(-2.0))", &[-1.0]),
        case("sign(F(0.0))", &[0.0]),
        case("sign(F(-0.0))", &[-0.0]),
        case("fma(F(2.0), F(3.0), F(4.0))", &[10.0]),
        case("mix(F(2.0), F(4.0), F(0.25))", &[2.5]),
        case(
            "mix(vec2<f32>(F(0.0), F(2.0)), vec2<f32>(F(4.0), F(6.0)), F(0.5))",
            &[2.0, 4.0],
        ),
        case("step(F(1.0), F(2.0))", &[1.0]),
        case("step(F(2.0), F(2.0))", &[1.0]),
        case("step(F(3.0), F(2.0))", &[0.0]),
        case("smoothstep(F(1.0), F(5.0), F(2.0))", &[0.15625]),
        case("sqrt(F(4.0))", &[2.0]),
        case("inverseSqrt(F(4.0))", &[0.5]),
        case("quantizeToF16(F(65519.0))", &[65504.0]),
        case("quantizeToF16(F(1.0001))", &[1.0]),
        // sin(1) = 0.84147…, to f16's 2^-11 steps in [0.5, 1): 1723 / 2048.
        case("sin(H(1.0))", &[0.841_308_593_75]),
    ]
}

/// The vector, matrix and decomposition built-ins.
fn vector_builtins() -> Vec<Case> {
    // refract((0.6, -0.8), (0, 1), 0.5) in f32, operation by operation.
    let (e1, d, eta) = ([0.6f32, -0.8], -0.8f32, 0.5f32);
    let k = 1.0 - eta * eta * (1.0 - d * d);
    let s = eta * d + k.sqrt();
    let refracted = [f64::from(eta * e1[0]), f64::from(eta * e1[1] - s)];
    let subnormal = "D(1.0) * 4.9406564584124654e-324lf";
    vec![
        case(
            "dot(vec2<f32>(F(1.0), F(2.0)), vec2<f32>(F(3.0), F(4.0)))",
            &[11.0],
        ),
        case(
            "f32(dot(vec2<i32>(I(1), I(-2)), vec2<i32>(I(3), I(4))))",
            &[-5.0],
        ),
        case(
            "cross(vec3<f32>(F(1.0), F(2.0), F(3.0)), vec3<f32>(F(4.0), F(5.0), F(6.0)))",
            &[-3.0, 6.0, -3.0],
        ),
        case("length(vec2<f32>(F(3.0), F(4.0)))", &[5.0]),
        case("length(F(-3.0))", &[3.0]),
        case(
            "distance(vec2<f32>(F(1.0), F(1.0)), vec2<f32>(F(4.0), F(5.0)))",
            &[5.0],
        ),
        case("distance(F(1.0), F(4.0))", &[3.0]),
        case(
            "normalize(vec2<f32>(F(3.0), F(4.0)))",
            &[r32(0.6), r32(0.8)],
        ),
        case(
            "faceForward(vec2<f32>(F(1.0), F(2.0)), vec2<f32>(F(1.0), F(0.0)), vec2<f32>(F(-1.0), F(0.0)))",
            &[1.0, 2.0],
        ),
        case(
            "faceForward(vec2<f32>(F(1.0), F(2.0)), vec2<f32>(F(1.0), F(0.0)), vec2<f32>(F(1.0), F(0.0)))",
            &[-1.0, -2.0],
        ),
        case(
            "faceForward(vec2<f32>(F(1.0), F(2.0)), vec2<f32>(F(1.0), F(0.0)), vec2<f32>(F(0.0), F(1.0)))",
            &[-1.0, -2.0],
        ),
        case(
            "reflect(vec2<f32>(F(1.0), F(-2.0)), vec2<f32>(F(0.0), F(1.0)))",
            &[1.0, 2.0],
        ),
        case(
            "refract(vec2<f32>(F(0.6), F(-0.8)), vec2<f32>(F(0.0), F(1.0)), F(0.5))",
            &refracted,
        ),
        case(
            "refract(vec2<f32>(F(0.6), F(-0.8)), vec2<f32>(F(0.0), F(1.0)), F(2.0))",
            &[0.0, 0.0],
        ),
        // k = 1 - 1 * (1 - 0) = 0 exactly: no total internal reflection.
        case(
            "refract(vec2<f32>(F(1.0), F(0.0)), vec2<f32>(F(0.0), F(1.0)), F(1.0))",
            &[1.0, 0.0],
        ),
        case(
            "transpose(mat2x3<f32>(vec3<f32>(F(1.0), F(2.0), F(3.0)), vec3<f32>(F(4.0), F(5.0), F(6.0))))",
            &[1.0, 4.0, 2.0, 5.0, 3.0, 6.0],
        ),
        case(&format!("determinant({A})"), &[-2.0]),
        case(
            "determinant(mat3x3<f32>(vec3<f32>(F(2.0), F(0.0), F(1.0)), vec3<f32>(F(1.0), F(3.0), F(2.0)), \
             vec3<f32>(F(1.0), F(1.0), F(4.0))))",
            &[18.0],
        ),
        case("modf(F(-1.25)).fract", &[-0.25]),
        case("modf(F(-1.25)).whole", &[-1.0]),
        case("modf(vec2<f32>(F(1.5), F(-2.25))).fract", &[0.5, -0.25]),
        case("frexp(F(12.0)).fract", &[0.75]),
        case("f32(frexp(F(12.0)).exp)", &[4.0]),
        case("frexp(F(-0.375)).fract", &[-0.75]),
        case("f32(frexp(F(-0.375)).exp)", &[-1.0]),
        case("frexp(F(0.0)).fract", &[0.0]),
        case("f32(frexp(F(0.0)).exp)", &[0.0]),
        case(&format!("frexp({subnormal}).fract"), &[0.5]),
        case(&format!("f64(frexp({subnormal}).exp)"), &[-1073.0]),
        case(
            "vec2<f32>(frexp(vec2<f32>(F(12.0), F(1.0))).exp)",
            &[4.0, 1.0],
        ),
        case("ldexp(F(1.5), I(3))", &[12.0]),
        case("ldexp(F(1.0), I(-3))", &[0.125]),
        case("ldexp(F(1.0), I(128))", &[f64::INFINITY]),
        case("ldexp(H(1.0), I(16))", &[f64::INFINITY]),
        case(&format!("ldexp({subnormal}, I(2000))"), &[2f64.powi(926)]),
        case(
            "ldexp(vec2<f32>(F(1.0), F(2.0)), vec2<i32>(I(1), I(2)))",
            &[2.0, 8.0],
        ),
    ]
}

/// The integer, bit and packing built-ins.
fn integer_builtins() -> Vec<Case> {
    let max = r32(f64::from(u32::MAX));
    let tiny = 2f64.powi(-24);
    let min_normal = 2f64.powi(-14);
    vec![
        case("f32(abs(I(-3)))", &[3.0]),
        case("f32(abs(U(3)))", &[3.0]),
        case("f32(min(I(-3), I(2)))", &[-3.0]),
        case("f32(max(I(-3), I(2)))", &[2.0]),
        case("f32(min(U(3), U(2)))", &[2.0]),
        case("f32(max(U(3), U(2)))", &[3.0]),
        case("f32(clamp(I(-5), I(-1), I(3)))", &[-1.0]),
        case("f32(clamp(U(5), U(1), U(3)))", &[3.0]),
        case("f32(sign(I(-5)))", &[-1.0]),
        case("f32(countTrailingZeros(U(8)))", &[3.0]),
        case("f32(countLeadingZeros(U(1)))", &[31.0]),
        case("f32(countOneBits(U(7)))", &[3.0]),
        case("f32(countOneBits(I(-1)))", &[32.0]),
        case("f32(reverseBits(U(1)))", &[2_147_483_648.0]),
        case("f32(firstTrailingBit(U(8)))", &[3.0]),
        case("f32(firstTrailingBit(U(0)))", &[max]),
        case("f32(firstTrailingBit(I(0)))", &[-1.0]),
        case("f32(firstLeadingBit(U(8)))", &[3.0]),
        case("f32(firstLeadingBit(U(0)))", &[max]),
        case("f32(firstLeadingBit(I(8)))", &[3.0]),
        case("f32(firstLeadingBit(I(-8)))", &[2.0]),
        case("f32(firstLeadingBit(I(0)))", &[-1.0]),
        case("f32(firstLeadingBit(I(-1)))", &[-1.0]),
        case("f32(extractBits(U(0x5a3), U(4), U(4)))", &[10.0]),
        case(
            "f32(extractBits(U(0xffffffff), U(4), U(100)))",
            &[r32(f64::from(0x0fff_ffffu32))],
        ),
        case("f32(extractBits(U(0x5a3), U(40), U(4)))", &[0.0]),
        case("f32(extractBits(I(0x5a3), U(4), U(4)))", &[-6.0]),
        case("f32(extractBits(I(0x573), U(4), U(4)))", &[7.0]),
        case("f32(extractBits(I(0x5a3), U(4), U(0)))", &[0.0]),
        case("f32(extractBits(I(0x5a3), U(0), U(32)))", &[1443.0]),
        case("f32(insertBits(U(0xffff), U(5), U(4), U(4)))", &[65375.0]),
        case("f32(insertBits(U(7), U(1), U(4), U(0)))", &[7.0]),
        case("f32(insertBits(I(-1), I(0), U(0), U(8)))", &[-256.0]),
        case(
            "f32(insertBits(U(0), U(0xffffffff), U(28), U(100)))",
            &[r32(f64::from(0xf000_0000u32))],
        ),
        case("f32(dot4I8Packed(U(0x0104fd02), U(0x08f90605)))", &[-28.0]),
        case("f32(dot4U8Packed(U(0x0104fd02), U(0x08f90605)))", &[2532.0]),
        case(
            &bytes("pack4x8snorm(vec4<f32>(F(1.0), F(-2.0), F(0.5), F(0.0)))"),
            &[127.0, 129.0, 64.0, 0.0],
        ),
        case(
            &bytes("pack4x8unorm(vec4<f32>(F(1.0), F(0.5), F(-1.0), F(0.2)))"),
            &[255.0, 128.0, 0.0, 51.0],
        ),
        case(
            &halves("pack2x16snorm(vec2<f32>(F(-1.0), F(0.5)))"),
            &[32769.0, 16384.0],
        ),
        case(
            &halves("pack2x16unorm(vec2<f32>(F(1.0), F(0.5)))"),
            &[65535.0, 32768.0],
        ),
        case(
            &halves("pack2x16float(vec2<f32>(F(1.5), F(-65504.0)))"),
            &[15872.0, 64511.0],
        ),
        case(
            &halves("pack2x16float(vec2<f32>(F(70000.0), F(NaN)))"),
            &[31744.0, 32256.0],
        ),
        case(
            &halves("pack2x16float(vec2<f32>(F(5.9604645e-8), F(6.1035156e-5)))"),
            &[1.0, 1024.0],
        ),
        case(
            &halves("pack2x16float(vec2<f32>(F(-0.0), F(0.0)))"),
            &[32768.0, 0.0],
        ),
        case(
            &bytes("pack4xI8(vec4<i32>(I(1), I(-1), I(127), I(-128)))"),
            &[1.0, 255.0, 127.0, 128.0],
        ),
        case(
            &bytes("pack4xU8(vec4<u32>(U(1), U(255), U(263), U(0)))"),
            &[1.0, 255.0, 7.0, 0.0],
        ),
        case(
            &bytes("pack4xI8Clamp(vec4<i32>(I(200), I(-200), I(5), I(-5)))"),
            &[127.0, 128.0, 5.0, 251.0],
        ),
        case(
            &bytes("pack4xU8Clamp(vec4<u32>(U(300), U(5), U(255), U(0)))"),
            &[255.0, 5.0, 255.0, 0.0],
        ),
        case(
            "unpack4x8snorm(U(0x80817f40))",
            &[r32(64.0 / 127.0), 1.0, -1.0, -1.0],
        ),
        case(
            "unpack4x8unorm(U(0x003380ff))",
            &[1.0, r32(128.0 / 255.0), r32(51.0 / 255.0), 0.0],
        ),
        case(
            "unpack2x16snorm(U(0x80004000))",
            &[r32(16384.0 / 32767.0), -1.0],
        ),
        case(
            "unpack2x16unorm(U(0xffff8000))",
            &[r32(32768.0 / 65535.0), 1.0],
        ),
        case("unpack2x16float(U(0x7c003e00))", &[1.5, f64::INFINITY]),
        case("unpack2x16float(U(0x00017e00))", &[f64::NAN, tiny]),
        case("unpack2x16float(U(0x8000fbff))", &[-65504.0, -0.0]),
        case("unpack2x16float(U(0x84000400))", &[min_normal, -min_normal]),
        case(
            "vec4<f32>(unpack4xI8(U(0x80ff7f01)))",
            &[1.0, 127.0, -1.0, -128.0],
        ),
        case(
            "vec4<f32>(unpack4xU8(U(0x80ff7f01)))",
            &[1.0, 127.0, 255.0, 128.0],
        ),
    ]
}

/// A NaN's bit pattern, known when a `bitcast` made it: through a `bitcast` back, a component, a swizzle, a `select`
/// or a `transpose`, its sign and payload kept, for f32, f16 (through u16 and i16) and f64 (through u64).
fn nan_bits() -> Vec<Case> {
    let f32_max = f64::from(f32::MAX);
    let below_max = f64::from(f32::from_bits(0x7f7f_fffe));
    vec![
        case("f32(bitcast<u32>(F(NaN)))", &[2_143_289_344.0]),
        case("f32(bitcast<u32>(bitcast<f32>(0xffc00001u)) >> 16u)", &[65472.0]),
        case("f32(bitcast<u32>(bitcast<f32>(0xffc00001u)) & 0xffffu)", &[1.0]),
        case("f32(bitcast<i32>(bitcast<f32>(0xffc00001u)))", &[-4_194_303.0]),
        case("bitcast<f32>(bitcast<u32>(bitcast<f32>(0xffc00001u)))", &[f64::NAN]),
        case("bitcast<f32>(bitcast<u32>(bitcast<f32>(0x7fc00000u)) & 0x7f800000u)", &[f64::INFINITY]),
        case("bitcast<f32>(bitcast<u32>(bitcast<f32>(0x7fc00000u)) - 0x00400001u)", &[f32_max]),
        case("bitcast<f32>(bitcast<u32>(bitcast<f32>(0x7fc00000u)) - 0x00400002u)", &[below_max]),
        case(
            "vec2<f32>(bitcast<vec2<u32>>(vec2<f32>(bitcast<f32>(0x7f800001u), F(1.0))))",
            &[r32(2_139_095_041.0), 1_065_353_216.0],
        ),
        case(
            "f32(bitcast<u32>(select(F(1.0), bitcast<f32>(0xff800001u), B(true))))",
            &[r32(4_286_578_689.0)],
        ),
        case(
            "f32(bitcast<vec2<u32>>(vec2<f32>(F(1.0), bitcast<f32>(0x7fc00003u)).yx).x & 0xffu)",
            &[3.0],
        ),
        case(
            "f32(bitcast<u32>(array<f32, 2>(F(1.0), bitcast<f32>(0x7fc00005u))[1]) & 0xffu)",
            &[5.0],
        ),
        case(
            "f32(bitcast<u32>(transpose(mat2x2<f32>(vec2<f32>(F(1.0), bitcast<f32>(0x7fc00007u)), \
             vec2<f32>(F(3.0), F(4.0))))[1][0]) & 0xffu)",
            &[7.0],
        ),
        case("bitcast<f16>(u16(0x7c00u))", &[f64::INFINITY]),
        case("bitcast<f16>(u16(0x7bffu))", &[65504.0]),
        case("f32(bitcast<u16>(bitcast<f16>(u16(0xfe01u))))", &[65025.0]),
        case("f32(bitcast<i16>(bitcast<f16>(u16(0xfe01u))))", &[-511.0]),
        case(
            "bitcast<f16>(bitcast<u16>(bitcast<f16>(u16(0x7e00u))) - u16(0x0201u))",
            &[65504.0],
        ),
        case(
            "bitcast<f16>(bitcast<u16>(bitcast<f16>(u16(0xfe00u))) & u16(0x7c00u))",
            &[f64::INFINITY],
        ),
        case(
            "bitcast<vec2<f16>>(bitcast<vec2<u16>>(vec2<f16>(bitcast<f16>(u16(0x7e01u)), H(1.0))) & \
             vec2<u16>(u16(0x7c00u)))",
            &[f64::INFINITY, 1.0],
        ),
        case("bitcast<f64>(0x7ff0000000000000lu)", &[f64::INFINITY]),
        case(
            "bitcast<f64>(bitcast<u64>(bitcast<f64>(0x7ff0000000000000lu)) - 1lu)",
            &[f64::MAX],
        ),
        case(
            "f64(bitcast<u64>(bitcast<f64>(0xfff8000000000003lu)) & 0xfflu)",
            &[3.0],
        ),
    ]
}

/// Integers of 16 and 64 bits, and unsigned division of u32s past i32's range: every operator and integer built-in,
/// and conversions, at their width.
fn wide_integers() -> Vec<Case> {
    let one_bits = 4_607_182_418_800_017_408.0; // 0x3ff0000000000000, f64 1.0's bits.
    vec![
        case("f32(countOneBits(bitcast<u64>(D(-1.0))))", &[11.0]),
        case("f32(firstLeadingBit(bitcast<i64>(D(-1.0))))", &[62.0]),
        case("f32(firstLeadingBit(bitcast<u64>(D(1.0))))", &[61.0]),
        case("f32(i32(bitcast<i64>(D(-1.0)) >> 60u))", &[-5.0]),
        case("f32(u32(bitcast<u64>(D(1.0)) >> 32u))", &[1_072_693_248.0]),
        case(
            "f32(extractBits(bitcast<u64>(D(1.0)), 52u, 11u))",
            &[1023.0],
        ),
        case("f32(extractBits(bitcast<i64>(D(-1.0)), 60u, 4u))", &[-5.0]),
        case("f32(extractBits(bitcast<i64>(D(-1.0)), 60u, 8u))", &[-5.0]),
        case(
            "f32(min(bitcast<i64>(D(-1.0)), bitcast<i64>(D(1.0))))",
            &[-4_616_189_618_054_758_400.0],
        ),
        case(
            "f32(max(bitcast<u64>(D(-1.0)), bitcast<u64>(D(1.0))))",
            &[13_830_554_455_654_793_216.0],
        ),
        case(
            "f32(insertBits(bitcast<u64>(D(1.0)), bitcast<u64>(D(1.0)) >> 52u, 0u, 4u) & 0xfflu)",
            &[15.0],
        ),
        case("f32(reverseBits(bitcast<u16>(H(1.0))))", &[60.0]),
        case("f32(countLeadingZeros(bitcast<u16>(H(1.0))))", &[2.0]),
        case("f32(countTrailingZeros(bitcast<u16>(H(1.0))))", &[10.0]),
        case("f32(firstTrailingBit(bitcast<u16>(H(1.0))))", &[10.0]),
        case("f32(-bitcast<i16>(H(-1.0)))", &[17408.0]),
        case("f32(~bitcast<u16>(H(1.0)))", &[50175.0]),
        case(
            "f32(abs(bitcast<i64>(D(-2.0))))",
            &[4_611_686_018_427_387_904.0],
        ),
        case(
            "f32(max(bitcast<i64>(D(-1.0)), bitcast<i64>(D(1.0))))",
            &[one_bits],
        ),
        case(
            "f32(min(bitcast<u64>(D(-1.0)), bitcast<u64>(D(1.0))))",
            &[one_bits],
        ),
        case("f32(sign(bitcast<i64>(D(-1.0))))", &[-1.0]),
        case("f32(i32(bitcast<i16>(H(-1.0))))", &[-17408.0]),
        case("f32(u32(bitcast<i16>(H(-1.0))))", &[4_294_949_888.0]),
        case("f32(u16(F(70000.0)))", &[65535.0]),
        case("f32(i16(F(-1.5)))", &[-1.0]),
        case("f32(u64(F(3.5)))", &[3.0]),
        case("f32(i64(F(-3.5)))", &[-3.0]),
        case(
            "f64(bitcast<i64>(D(-1.0)))",
            &[-4_616_189_618_054_758_400.0],
        ),
        case("f16(bitcast<i16>(H(-1.0)))", &[-17408.0]),
        case("f32(bitcast<i64>(D(1.0)) % 1000li)", &[408.0]),
        case("f32(U(0xffffffff) / U(2))", &[r32(2_147_483_647.0)]),
        case("f32(U(0xffffffff) % U(10))", &[5.0]),
        case(
            "f32(bitcast<u64>(D(-1.0)) / 2lu)",
            &[6_915_277_227_827_396_608.0],
        ),
        case("f32(bitcast<u64>(D(-1.0)) % 7lu)", &[3.0]),
        // 2^60 + 2^36 + 1, rounded once to f32: up, to 2^60 + 2^37 (by way of an f64, it would tie and round down).
        case(
            "f32(bitcast<u64>(D(0.0)) + 0x1000001000000001lu)",
            &[1_152_921_642_045_800_448.0],
        ),
        case(
            "f32(bitcast<i64>(D(0.0)) + 0x1000001000000001li)",
            &[1_152_921_642_045_800_448.0],
        ),
        case("f32(bitcast<i16>(H(-1.0)) / i16(-1))", &[17408.0]),
        case("f32(bitcast<i16>(H(-2.0)) * i16(3))", &[16384.0]),
        case("f32(bitcast<u64>(D(1.0)) % 1000lu)", &[408.0]),
        case("f32(bitcast<i64>(D(-1.0)) < bitcast<i64>(D(1.0)))", &[1.0]),
        case("f32(bitcast<u64>(D(-1.0)) < bitcast<u64>(D(1.0)))", &[0.0]),
        case(
            "f64(bitcast<u64>(D(-1.0)))",
            &[13_830_554_455_654_793_216.0],
        ),
        case("f16(bitcast<u16>(H(1.0)))", &[15360.0]),
        case(
            "vec2<f32>(F(1.0), F(2.0))[bitcast<u64>(D(0.0)) + 1lu]",
            &[2.0],
        ),
    ]
}

/// Values WGSL leaves indeterminate, or that divide by zero: none. A NaN an operator, a conversion or a built-in
/// computes has indeterminate bits, so a `bitcast` of it is none too.
fn indeterminate() -> Vec<Case> {
    [
        "frexp(F(inf)).fract",
        "smoothstep(F(1.0), F(1.0), F(2.0))",
        "f32(u32(F(NaN)))",
        "f32(i32(F(NaN)))",
        "f32(bitcast<u32>(F(inf) - F(inf)))",
        "f32(bitcast<u32>(-bitcast<f32>(0x7fc00000u)))",
        "f32(bitcast<u32>(abs(F(NaN))))",
        "f32(bitcast<u64>(D(NaN)))",
    ]
    .iter()
    .map(|e| (e.to_string(), None))
    .collect()
}

/// The `indeterminate` cases' defined neighbours, each with a value.
#[cfg(feature = "controls")]
fn determinate() -> Vec<Case> {
    [
        "frexp(F(2.0)).fract",
        "smoothstep(F(1.0), F(2.0), F(2.0))",
        "f32(u32(F(1.0)))",
        "f32(i32(F(1.0)))",
        "f32(bitcast<u32>(F(inf) - F(1.0)))",
        "f32(bitcast<u32>(-bitcast<f32>(0x7f800000u)))",
        "f32(bitcast<u32>(abs(F(-1.0))))",
        "f32(bitcast<u64>(D(1.0)))",
    ]
    .iter()
    .map(|e| (e.to_string(), None))
    .collect()
}

#[test]
fn lint_wgsl_finite_max_eval_operators() {
    check(&operators());
}

negative_control!(
    lint_wgsl_finite_max_eval_operators,
    "operators against perturbed values mismatch",
    expected = "mismatched",
    check(&perturbed(&operators()))
);

#[test]
fn lint_wgsl_finite_max_eval_float_builtins() {
    check(&float_builtins());
}

negative_control!(
    lint_wgsl_finite_max_eval_float_builtins,
    "float built-ins against perturbed values mismatch",
    expected = "mismatched",
    check(&perturbed(&float_builtins()))
);

#[test]
fn lint_wgsl_finite_max_eval_vector_builtins() {
    check(&vector_builtins());
}

negative_control!(
    lint_wgsl_finite_max_eval_vector_builtins,
    "vector built-ins against perturbed values mismatch",
    expected = "mismatched",
    check(&perturbed(&vector_builtins()))
);

#[test]
fn lint_wgsl_finite_max_eval_integer_builtins() {
    check(&integer_builtins());
}

negative_control!(
    lint_wgsl_finite_max_eval_integer_builtins,
    "integer built-ins against perturbed values mismatch",
    expected = "mismatched",
    check(&perturbed(&integer_builtins()))
);

#[test]
fn lint_wgsl_finite_max_eval_nan_bits() {
    check(&nan_bits());
}

negative_control!(
    lint_wgsl_finite_max_eval_nan_bits,
    "NaN bit patterns against perturbed values mismatch",
    expected = "mismatched",
    check(&perturbed(&nan_bits()))
);

#[test]
fn lint_wgsl_finite_max_eval_wide_integers() {
    check(&wide_integers());
}

negative_control!(
    lint_wgsl_finite_max_eval_wide_integers,
    "16- and 64-bit integers against perturbed values mismatch",
    expected = "mismatched",
    check(&perturbed(&wide_integers()))
);

#[test]
fn lint_wgsl_finite_max_eval_indeterminate_is_none() {
    check(&indeterminate());
}

negative_control!(
    lint_wgsl_finite_max_eval_indeterminate_is_none,
    "the defined neighbours of indeterminate values have values",
    expected = "mismatched",
    check(&determinate())
);
