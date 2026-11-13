//! `Real`'s transcendentals at each precision (R-991, REQ-CHART-212):
//! - `real_pinned_bits`: at f64 each method gives the pinned libm's result, bit for bit, and the bits recorded here
//!   from libm 0.2.16 on the Mac, on fixed inputs, so that on every machine the f64 path is the pinned library and
//!   the pinned library gives the Mac's bits; `sine_cosine` is `(sine, cosine)` and `sinc` `sine(r)/r`. Every
//!   function's inputs hold at least one where the Mac's platform library and libm differ (marked in the table, found
//!   by a sweep of 10⁶ inputs per function), and `sine`'s four: at `0xc021cb0947a686b4` and `0xbfe2b074a5ac7510` libm's
//!   `sin` is correctly rounded and the Mac's is not, at `0xc023fa30a11813de` and `0xc0155b4e049f4aee` the Mac's is and
//!   libm's is not (checked at 200 bits with mpmath), so a correctly rounded platform `sin`, as glibc's, differs too.
//!   So a method left on std fails here. Its control is that method, `sine` on `f64::sin`.
//! - `real_pinned_bits_f32`: at f32, the GPU's precision, each method is `num_traits::Float`'s, unchanged (R-991
//!   pins the f64 path alone); `arctan2` and `sinc` are the kernel's own, held by their own tests. Its control,
//!   `exp_minus_one` as `exp(x) − 1`, loses `x` near 0.

use kernel::chart::Float;
use kernel::Real;
use validation::negative_control;

/// A one-argument method and its pinned libm function.
type Unary<T> = fn(T) -> T;

/// A two-argument method and its pinned libm function.
type Binary<T> = fn(T, T) -> T;

/// `Real`'s transcendentals at a float type, as functions, so a control can put another in one's place.
#[derive(Clone, Copy)]
struct Methods<T> {
    unary: [(&'static str, Unary<T>); 14],
    binary: [(&'static str, Binary<T>); 4],
    sine_cosine: fn(T) -> (T, T),
    sinc: Binary<T>,
}

/// `Real`'s at `T`.
fn real<T: Real + Float>() -> Methods<T> {
    Methods {
        unary: [
            ("sine", |x: T| x.sine()),
            ("cosine", |x: T| x.cosine()),
            ("tangent", |x: T| x.tangent()),
            ("arcsine", |x: T| x.arcsine()),
            ("arccosine", |x: T| x.arccosine()),
            ("exponential", |x: T| x.exponential()),
            ("exp_minus_one", |x: T| x.exp_minus_one()),
            ("logarithm", |x: T| x.logarithm()),
            ("ln_one_plus", |x: T| x.ln_one_plus()),
            ("cube_root", |x: T| x.cube_root()),
            ("hyperbolic_sine", |x: T| x.hyperbolic_sine()),
            ("hyperbolic_cosine", |x: T| x.hyperbolic_cosine()),
            ("hyperbolic_tangent", |x: T| x.hyperbolic_tangent()),
            ("arctanh", |x: T| x.arctanh()),
        ],
        binary: [
            ("arctan2", |y: T, x: T| y.arctan2(x)),
            ("arctangent2", |y: T, x: T| y.arctangent2(x)),
            ("power", |b: T, e: T| b.power(e)),
            ("hypotenuse", |a: T, b: T| a.hypotenuse(b)),
        ],
        sine_cosine: |x: T| x.sine_cosine(),
        sinc: |r: T, square: T| r.sinc(square),
    }
}

/// The pinned libm's function for each of [`Methods`]'s, by name.
fn libm_unary(name: &str) -> Unary<f64> {
    match name {
        "sine" => libm::sin,
        "cosine" => libm::cos,
        "tangent" => libm::tan,
        "arcsine" => libm::asin,
        "arccosine" => libm::acos,
        "exponential" => libm::exp,
        "exp_minus_one" => libm::expm1,
        "logarithm" => libm::log,
        "ln_one_plus" => libm::log1p,
        "cube_root" => libm::cbrt,
        "hyperbolic_sine" => libm::sinh,
        "hyperbolic_cosine" => libm::cosh,
        "hyperbolic_tangent" => libm::tanh,
        "arctanh" => libm::atanh,
        other => panic!("no libm function for {other}"),
    }
}

/// The pinned libm's two-argument function, by name.
fn libm_binary(name: &str) -> Binary<f64> {
    match name {
        "arctan2" | "arctangent2" => libm::atan2,
        "power" => libm::pow,
        "hypotenuse" => libm::hypot,
        other => panic!("no libm function for {other}"),
    }
}

/// `num_traits::Float`'s function at f32 for each of [`Methods`]'s, by name.
fn float_unary(name: &str) -> Unary<f32> {
    match name {
        "sine" => Float::sin,
        "cosine" => Float::cos,
        "tangent" => Float::tan,
        "arcsine" => Float::asin,
        "arccosine" => Float::acos,
        "exponential" => Float::exp,
        "exp_minus_one" => Float::exp_m1,
        "logarithm" => Float::ln,
        "ln_one_plus" => Float::ln_1p,
        "cube_root" => Float::cbrt,
        "hyperbolic_sine" => Float::sinh,
        "hyperbolic_cosine" => Float::cosh,
        "hyperbolic_tangent" => Float::tanh,
        "arctanh" => Float::atanh,
        other => panic!("no Float function for {other}"),
    }
}

/// libm 0.2.16's results on the Mac, `(input, result)` bits, per unary method in [`Methods`]'s order (generated once
/// from `libm::<function>`; the inputs marked are where the Mac's platform library differs).
const UNARY: [(&str, &[(u64, u64)]); 14] = [
    (
        "sine",
        &[
            (0x0000000000000000, 0x0000000000000000),
            (0x8000000000000000, 0x8000000000000000),
            (0x01a56e1fc2f8f359, 0x01a56e1fc2f8f359),
            (0x3e45798ee2308c3a, 0x3e45798ee2308c3a),
            (0x3fd0000000000000, 0x3fcfaaeed4f31577),
            (0x3fe0000000000000, 0x3fdeaee8744b05f0),
            (0x3fe8000000000000, 0x3fe5cffc16bf8f0d),
            (0x3ff0000000000000, 0x3feaed548f090cee),
            (0x3ff8000000000000, 0x3fefeb7a9b2c6d8b),
            (0x4000000000000000, 0x3fed18f6ead1b446),
            (0x4008000000000000, 0x3fc210386db6d55b),
            (0x4024000000000000, 0xbfe1689ef5f34f52),
            (0x4059000000000000, 0xbfe03425b78c4db8),
            (0x412e848000000000, 0xbfd6664b2568d867),
            (0xbfe0000000000000, 0xbfdeaee8744b05f0),
            (0xc000000000000000, 0xbfed18f6ead1b446),
            (0xc021cb0947a686b4, 0xbfe020c2d3b35ebb), // the Mac's platform library differs
            (0xbfe2b074a5ac7510, 0xbfe1a50f040a5385), // the Mac's platform library differs
            (0xc023fa30a11813de, 0x3fe11a55c41a3928), // the Mac's platform library differs
            (0xc0155b4e049f4aee, 0x3fe9eae33f0154cd), // the Mac's platform library differs
        ],
    ),
    (
        "cosine",
        &[
            (0x0000000000000000, 0x3ff0000000000000),
            (0x8000000000000000, 0x3ff0000000000000),
            (0x01a56e1fc2f8f359, 0x3ff0000000000000),
            (0x3e45798ee2308c3a, 0x3ff0000000000000),
            (0x3fd0000000000000, 0x3fef01549f7deea1),
            (0x3fe0000000000000, 0x3fec1528065b7d50),
            (0x3fe8000000000000, 0x3fe769fec655211f),
            (0x3ff0000000000000, 0x3fe14a280fb5068c),
            (0x3ff8000000000000, 0x3fb21bd54fc5f9a7),
            (0x4000000000000000, 0xbfdaa22657537205),
            (0x4008000000000000, 0xbfefae04be85e5d2),
            (0x4024000000000000, 0xbfead9ac890c6b1f),
            (0x4059000000000000, 0x3feb981dbf665fdf),
            (0x412e848000000000, 0x3fedf9df9906d32c),
            (0xbfe0000000000000, 0x3fec1528065b7d50),
            (0xc000000000000000, 0xbfdaa22657537205),
            (0x401aa809f48e44c0, 0x3fedb4d6318fd061), // the Mac's platform library differs
        ],
    ),
    (
        "tangent",
        &[
            (0x0000000000000000, 0x0000000000000000),
            (0x8000000000000000, 0x8000000000000000),
            (0x01a56e1fc2f8f359, 0x01a56e1fc2f8f359),
            (0x3e45798ee2308c3a, 0x3e45798ee2308c3a),
            (0x3fd0000000000000, 0x3fd05785a43c4c56),
            (0x3fe0000000000000, 0x3fe17b4f5bf3474a),
            (0x3fe8000000000000, 0x3fedcfa36110eeec), // the Mac's platform library differs
            (0x3ff0000000000000, 0x3ff8eb245cbee3a6), // the Mac's platform library differs
            (0x3ff8000000000000, 0x402c33ed50b88777), // the Mac's platform library differs
            (0x4000000000000000, 0xc0017af62e0950f8),
            (0x4008000000000000, 0xbfc23ef71254b86f),
            (0x4024000000000000, 0x3fe4bf5f34be3782), // the Mac's platform library differs
            (0x4059000000000000, 0xbfe2ca74d62b5d38),
            (0x412e848000000000, 0xbfd7e9768ab734c0),
            (0xbfe0000000000000, 0xbfe17b4f5bf3474a),
            (0xc000000000000000, 0x40017af62e0950f8),
            (0xbfe6807a1223d8e3, 0xbfeb20b934050192), // the Mac's platform library differs
        ],
    ),
    (
        "arcsine",
        &[
            (0x0000000000000000, 0x0000000000000000),
            (0x8000000000000000, 0x8000000000000000),
            (0x01a56e1fc2f8f359, 0x01a56e1fc2f8f359),
            (0x3e45798ee2308c3a, 0x3e45798ee2308c3a),
            (0x3fd0000000000000, 0x3fd02be9ce0b87cd),
            (0x3fe0000000000000, 0x3fe0c152382d7366), // the Mac's platform library differs
            (0x3fe8000000000000, 0x3feb235315c680dc),
            (0x3ff0000000000000, 0x3ff921fb54442d18),
            (0x3ff8000000000000, 0x7ff8000000000000),
            (0x4000000000000000, 0x7ff8000000000000),
            (0x4008000000000000, 0x7ff8000000000000),
            (0x4024000000000000, 0x7ff8000000000000),
            (0x4059000000000000, 0x7ff8000000000000),
            (0x412e848000000000, 0x7ff8000000000000),
            (0xbfe0000000000000, 0xbfe0c152382d7366), // the Mac's platform library differs
            (0xc000000000000000, 0x7ff8000000000000), // the Mac's platform library differs
            (0xbfe97ef9b282bd3e, 0xbfed802fdb84e5ba), // the Mac's platform library differs
        ],
    ),
    (
        "arccosine",
        &[
            (0x0000000000000000, 0x3ff921fb54442d18),
            (0x8000000000000000, 0x3ff921fb54442d18),
            (0x01a56e1fc2f8f359, 0x3ff921fb54442d18),
            (0x3e45798ee2308c3a, 0x3ff921fb5194fb3c),
            (0x3fd0000000000000, 0x3ff51700e0c14b25),
            (0x3fe0000000000000, 0x3ff0c152382d7366), // the Mac's platform library differs
            (0x3fe8000000000000, 0x3fe720a392c1d955),
            (0x3ff0000000000000, 0x0000000000000000),
            (0x3ff8000000000000, 0x7ff8000000000000),
            (0x4000000000000000, 0x7ff8000000000000),
            (0x4008000000000000, 0x7ff8000000000000),
            (0x4024000000000000, 0x7ff8000000000000),
            (0x4059000000000000, 0x7ff8000000000000),
            (0x412e848000000000, 0x7ff8000000000000),
            (0xbfe0000000000000, 0x4000c152382d7366),
            (0xc000000000000000, 0x7ff8000000000000),
            (0x3fd8e98ecd26940c, 0x3ff2bc4e9b7d442f), // the Mac's platform library differs
        ],
    ),
    (
        "exponential",
        &[
            (0x0000000000000000, 0x3ff0000000000000),
            (0x8000000000000000, 0x3ff0000000000000),
            (0x01a56e1fc2f8f359, 0x3ff0000000000000),
            (0x3e45798ee2308c3a, 0x3ff0000002af31dc),
            (0x3fd0000000000000, 0x3ff48b5e3c3e8186),
            (0x3fe0000000000000, 0x3ffa61298e1e069c),
            (0x3fe8000000000000, 0x4000ef9db467dcf8),
            (0x3ff0000000000000, 0x4005bf0a8b14576a), // the Mac's platform library differs
            (0x3ff8000000000000, 0x4011ed3fe64fc541),
            (0x4000000000000000, 0x401d8e64b8d4ddae),
            (0x4008000000000000, 0x403415e5bf6fb106),
            (0x4024000000000000, 0x40d5829dcf950560),
            (0x4059000000000000, 0x48f3494a9b171bf5),
            (0x412e848000000000, 0x7ff0000000000000),
            (0xbfe0000000000000, 0x3fe368b2fc6f960a),
            (0xc000000000000000, 0x3fc152aaa3bf81cc),
            (0x4014921e47f59e4c, 0x406565a3b7001356), // the Mac's platform library differs
        ],
    ),
    (
        "exp_minus_one",
        &[
            (0x0000000000000000, 0x0000000000000000),
            (0x8000000000000000, 0x8000000000000000),
            (0x01a56e1fc2f8f359, 0x01a56e1fc2f8f359),
            (0x3e45798ee2308c3a, 0x3e45798ee3fdb764),
            (0x3fd0000000000000, 0x3fd22d78f0fa061a),
            (0x3fe0000000000000, 0x3fe4c2531c3c0d38),
            (0x3fe8000000000000, 0x3ff1df3b68cfb9f0), // the Mac's platform library differs
            (0x3ff0000000000000, 0x3ffb7e151628aed2), // the Mac's platform library differs
            (0x3ff8000000000000, 0x400bda7fcc9f8a83),
            (0x4000000000000000, 0x40198e64b8d4ddae),
            (0x4008000000000000, 0x403315e5bf6fb106),
            (0x4024000000000000, 0x40d5825dcf950560),
            (0x4059000000000000, 0x48f3494a9b171bf5),
            (0x412e848000000000, 0x7ff0000000000000),
            (0xbfe0000000000000, 0xbfd92e9a0720d3ec),
            (0xc000000000000000, 0xbfebab5557101f8d),
            (0xbffbb67dce9792bc, 0xbfea56aa5d400d78), // the Mac's platform library differs
        ],
    ),
    (
        "logarithm",
        &[
            (0x0000000000000000, 0xfff0000000000000),
            (0x8000000000000000, 0xfff0000000000000),
            (0x01a56e1fc2f8f359, 0xc085963447f87fb5),
            (0x3e45798ee2308c3a, 0xc0326bb1bbb55516),
            (0x3fd0000000000000, 0xbff62e42fefa39ef),
            (0x3fe0000000000000, 0xbfe62e42fefa39ef),
            (0x3fe8000000000000, 0xbfd269621134db92),
            (0x3ff0000000000000, 0x0000000000000000),
            (0x3ff8000000000000, 0x3fd9f323ecbf984c),
            (0x4000000000000000, 0x3fe62e42fefa39ef),
            (0x4008000000000000, 0x3ff193ea7aad030a), // the Mac's platform library differs
            (0x4024000000000000, 0x40026bb1bbb55516),
            (0x4059000000000000, 0x40126bb1bbb55516),
            (0x412e848000000000, 0x402ba18a998fffa0),
            (0xbfe0000000000000, 0x7ff8000000000000),
            (0xc000000000000000, 0x7ff8000000000000),
            (0x3fe4698b83c5dcf8, 0xbfdcc63ac5fb2370), // the Mac's platform library differs
        ],
    ),
    (
        "ln_one_plus",
        &[
            (0x0000000000000000, 0x0000000000000000),
            (0x8000000000000000, 0x8000000000000000),
            (0x01a56e1fc2f8f359, 0x01a56e1fc2f8f359),
            (0x3e45798ee2308c3a, 0x3e45798ee0636111),
            (0x3fd0000000000000, 0x3fcc8ff7c79a9a22),
            (0x3fe0000000000000, 0x3fd9f323ecbf984c),
            (0x3fe8000000000000, 0x3fe1e85f5e7040d0),
            (0x3ff0000000000000, 0x3fe62e42fefa39ef),
            (0x3ff8000000000000, 0x3fed5240f0e0e078),
            (0x4000000000000000, 0x3ff193ea7aad030a), // the Mac's platform library differs
            (0x4008000000000000, 0x3ff62e42fefa39ef),
            (0x4024000000000000, 0x40032ee3b77f374c),
            (0x4059000000000000, 0x401275e2271bba31),
            (0x412e848000000000, 0x402ba18abb1dedc8),
            (0xbfe0000000000000, 0xbfe62e42fefa39ef),
            (0xc000000000000000, 0x7ff8000000000000),
            (0xbfc76238a9001354, 0xbfc9d24f8a900a5c), // the Mac's platform library differs
        ],
    ),
    (
        "cube_root",
        &[
            (0x0000000000000000, 0x0000000000000000),
            (0x8000000000000000, 0x8000000000000000),
            (0x01a56e1fc2f8f359, 0x2b2bff2ee48e0530),
            (0x3e45798ee2308c3a, 0x3f61a62d511f2b53),
            (0x3fd0000000000000, 0x3fe428a2f98d728b),
            (0x3fe0000000000000, 0x3fe965fea53d6e3d),
            (0x3fe8000000000000, 0x3fed12ed0af1a27f),
            (0x3ff0000000000000, 0x3ff0000000000000),
            (0x3ff8000000000000, 0x3ff250bfe1b082f5),
            (0x4000000000000000, 0x3ff428a2f98d728b),
            (0x4008000000000000, 0x3ff7137449123ef6),
            (0x4024000000000000, 0x40013c484138704f),
            (0x4059000000000000, 0x401290fca9c761f8),
            (0x412e848000000000, 0x4059000000000000),
            (0xbfe0000000000000, 0xbfe965fea53d6e3d),
            (0xc000000000000000, 0xbff428a2f98d728b),
            (0xc04cb26acf1d40a0, 0xc00edbd7f4f075e8), // the Mac's platform library differs
        ],
    ),
    (
        "hyperbolic_sine",
        &[
            (0x0000000000000000, 0x0000000000000000),
            (0x8000000000000000, 0x8000000000000000),
            (0x01a56e1fc2f8f359, 0x01a56e1fc2f8f359),
            (0x3e45798ee2308c3a, 0x3e45798ee2308c3a),
            (0x3fd0000000000000, 0x3fd02accd9d08102),
            (0x3fe0000000000000, 0x3fe0acd00fe63b97),
            (0x3fe8000000000000, 0x3fea506b2dd3c690),
            (0x3ff0000000000000, 0x3ff2cd9fc44eb982),
            (0x3ff8000000000000, 0x400108c3aabd6a60),
            (0x4000000000000000, 0x400d03cf63b6e1a0), // the Mac's platform library differs
            (0x4008000000000000, 0x40240926e70949ae),
            (0x4024000000000000, 0x40c5829dced69992),
            (0x4059000000000000, 0x48e3494a9b171bf5),
            (0x412e848000000000, 0x7ff0000000000000),
            (0xbfe0000000000000, 0xbfe0acd00fe63b97),
            (0xc000000000000000, 0xc00d03cf63b6e1a0), // the Mac's platform library differs
            (0xbff238934e1fc32c, 0xbff66c40d12aa0b3), // the Mac's platform library differs
        ],
    ),
    (
        "hyperbolic_cosine",
        &[
            (0x0000000000000000, 0x3ff0000000000000),
            (0x8000000000000000, 0x3ff0000000000000),
            (0x01a56e1fc2f8f359, 0x3ff0000000000000),
            (0x3e45798ee2308c3a, 0x3ff0000000000000),
            (0x3fd0000000000000, 0x3ff080ab05ca6146),
            (0x3fe0000000000000, 0x3ff20ac1862ae8d0),
            (0x3fe8000000000000, 0x3ff4b705d1e5d6a8),
            (0x3ff0000000000000, 0x3ff8b07551d9f551), // the Mac's platform library differs
            (0x3ff8000000000000, 0x4002d1bc21e22022),
            (0x4000000000000000, 0x400e18fa0df2d9bc),
            (0x4008000000000000, 0x402422a497d6185e),
            (0x4024000000000000, 0x40c5829dd053712e), // the Mac's platform library differs
            (0x4059000000000000, 0x48e3494a9b171bf5),
            (0x412e848000000000, 0x7ff0000000000000),
            (0xbfe0000000000000, 0x3ff20ac1862ae8d0),
            (0xc000000000000000, 0x400e18fa0df2d9bc),
            (0xbffe862b322b091a, 0x400b8ba34f3e0a8f), // the Mac's platform library differs
        ],
    ),
    (
        "hyperbolic_tangent",
        &[
            (0x0000000000000000, 0x0000000000000000),
            (0x8000000000000000, 0x8000000000000000),
            (0x01a56e1fc2f8f359, 0x01a56e1fc2f8f359),
            (0x3e45798ee2308c3a, 0x3e45798ee2308c3a),
            (0x3fd0000000000000, 0x3fcf597ea69a1c86),
            (0x3fe0000000000000, 0x3fdd9353d7568af3),
            (0x3fe8000000000000, 0x3fe45323e552f228),
            (0x3ff0000000000000, 0x3fe85efab514f394),
            (0x3ff8000000000000, 0x3fecf6f9786df577),
            (0x4000000000000000, 0x3feed9505e1bc3d4),
            (0x4008000000000000, 0x3fefd77d111a0b00),
            (0x4024000000000000, 0x3feffffffdc96f35),
            (0x4059000000000000, 0x3ff0000000000000),
            (0x412e848000000000, 0x3ff0000000000000),
            (0xbfe0000000000000, 0xbfdd9353d7568af3),
            (0xc000000000000000, 0xbfeed9505e1bc3d4),
            (0xbfe7922107ee8618, 0xbfe4110898d6bfa0), // the Mac's platform library differs
        ],
    ),
    (
        "arctanh",
        &[
            (0x0000000000000000, 0x0000000000000000),
            (0x8000000000000000, 0x8000000000000000),
            (0x01a56e1fc2f8f359, 0x01a56e1fc2f8f359),
            (0x3e45798ee2308c3a, 0x3e45798ee2308c3b),
            (0x3fd0000000000000, 0x3fd058aefa811452), // the Mac's platform library differs
            (0x3fe0000000000000, 0x3fe193ea7aad030a), // the Mac's platform library differs
            (0x3fe8000000000000, 0x3fef2272ae325a57),
            (0x3ff0000000000000, 0x7ff0000000000000),
            (0x3ff8000000000000, 0x7ff8000000000000),
            (0x4000000000000000, 0x7ff8000000000000),
            (0x4008000000000000, 0x7ff8000000000000),
            (0x4024000000000000, 0x7ff8000000000000),
            (0x4059000000000000, 0x7ff8000000000000),
            (0x412e848000000000, 0x7ff8000000000000),
            (0xbfe0000000000000, 0xbfe193ea7aad030a),
            (0xc000000000000000, 0xfff8000000000000), // the Mac's platform library differs
            (0xbfef2b6345cb07b4, 0xc001537c4c5c84c2), // the Mac's platform library differs
        ],
    ),
];

/// A two-argument method's input and result bits, `(a, b, result)`.
type Pair = (u64, u64, u64);

/// libm 0.2.16's results on the Mac, `(a, b, result)` bits, per binary method in [`Methods`]'s order.
const BINARY: [(&str, &[Pair]); 4] = [
    (
        "arctan2",
        &[
            (0x0000000000000000, 0x3ff0000000000000, 0x0000000000000000),
            (0x3ff0000000000000, 0x0000000000000000, 0x3ff921fb54442d18),
            (0x3ff0000000000000, 0x3ff0000000000000, 0x3fe921fb54442d18),
            (0xbff0000000000000, 0xbff0000000000000, 0xc002d97c7f3321d2),
            (0x3fe0000000000000, 0x4000000000000000, 0x3fcf5b75f92c80dd),
            (0x4000000000000000, 0x3fe0000000000000, 0x3ff5368c951e9cfd),
            (0x4008000000000000, 0xc010000000000000, 0x4003fc176b7a8560),
            (0x3e45798ee2308c3a, 0x3ff0000000000000, 0x3e45798ee2308c3a),
            (0x4024000000000000, 0x4008000000000000, 0x3ff4782cbabc8157),
            (0x3fe8000000000000, 0x3ff8000000000000, 0x3fddac670561bb4f),
            (0x401364939f24f986, 0xbff06cb5a026bb80, 0x3ffc78a0c1c91779), // the Mac's platform library differs
        ],
    ),
    (
        "arctangent2",
        &[
            (0x0000000000000000, 0x3ff0000000000000, 0x0000000000000000),
            (0x3ff0000000000000, 0x0000000000000000, 0x3ff921fb54442d18),
            (0x3ff0000000000000, 0x3ff0000000000000, 0x3fe921fb54442d18),
            (0xbff0000000000000, 0xbff0000000000000, 0xc002d97c7f3321d2),
            (0x3fe0000000000000, 0x4000000000000000, 0x3fcf5b75f92c80dd),
            (0x4000000000000000, 0x3fe0000000000000, 0x3ff5368c951e9cfd),
            (0x4008000000000000, 0xc010000000000000, 0x4003fc176b7a8560),
            (0x3e45798ee2308c3a, 0x3ff0000000000000, 0x3e45798ee2308c3a),
            (0x4024000000000000, 0x4008000000000000, 0x3ff4782cbabc8157),
            (0x3fe8000000000000, 0x3ff8000000000000, 0x3fddac670561bb4f),
            (0x401364939f24f986, 0xbff06cb5a026bb80, 0x3ffc78a0c1c91779), // the Mac's platform library differs
        ],
    ),
    (
        "power",
        &[
            (0x0000000000000000, 0x3ff0000000000000, 0x0000000000000000),
            (0x3ff0000000000000, 0x0000000000000000, 0x3ff0000000000000),
            (0x3ff0000000000000, 0x3ff0000000000000, 0x3ff0000000000000),
            (0xbff0000000000000, 0xbff0000000000000, 0xbff0000000000000),
            (0x3fe0000000000000, 0x4000000000000000, 0x3fd0000000000000),
            (0x4000000000000000, 0x3fe0000000000000, 0x3ff6a09e667f3bcd),
            (0x4008000000000000, 0xc010000000000000, 0x3f8948b0fcd6e9e0),
            (0x3e45798ee2308c3a, 0x3ff0000000000000, 0x3e45798ee2308c3a),
            (0x4024000000000000, 0x4008000000000000, 0x408f400000000000),
            (0x3fe8000000000000, 0x3ff8000000000000, 0x3fe4c8dc2e423980),
            (0x400ab83c657556e7, 0xbfeddf03f44f7b7c, 0x3fd4c338eede9324), // the Mac's platform library differs
        ],
    ),
    (
        "hypotenuse",
        &[
            (0x0000000000000000, 0x3ff0000000000000, 0x3ff0000000000000),
            (0x3ff0000000000000, 0x0000000000000000, 0x3ff0000000000000),
            (0x3ff0000000000000, 0x3ff0000000000000, 0x3ff6a09e667f3bcd),
            (0xbff0000000000000, 0xbff0000000000000, 0x3ff6a09e667f3bcd),
            (0x3fe0000000000000, 0x4000000000000000, 0x40007e0f66afed07),
            (0x4000000000000000, 0x3fe0000000000000, 0x40007e0f66afed07),
            (0x4008000000000000, 0xc010000000000000, 0x4014000000000000),
            (0x3e45798ee2308c3a, 0x3ff0000000000000, 0x3ff0000000000000),
            (0x4024000000000000, 0x4008000000000000, 0x4024e16fdacff937),
            (0x3fe8000000000000, 0x3ff8000000000000, 0x3ffad5336963eefc),
            (0x401df30af6b9fe9a, 0xbfff86df9045b5ec, 0x401ef8190213263c), // the Mac's platform library differs
        ],
    ),
];

/// Checks `m`, the f64 methods, against the pinned libm and the recorded bits.
fn check_f64(m: &Methods<f64>) {
    let f = f64::from_bits;
    for ((name, method), (table_name, rows)) in m.unary.iter().zip(UNARY) {
        assert_eq!(*name, table_name, "the table's order");
        assert!(!rows.is_empty(), "{name} has inputs");
        for &(x, want) in rows {
            let got = method(f(x)).to_bits();
            assert_eq!(
                got,
                libm_unary(name)(f(x)).to_bits(),
                "{name}({x:#018x}) is libm's"
            );
            assert_eq!(got, want, "{name}({x:#018x}) is the Mac's libm bits");
        }
    }
    for ((name, method), (table_name, rows)) in m.binary.iter().zip(BINARY) {
        assert_eq!(*name, table_name, "the table's order");
        assert!(!rows.is_empty(), "{name} has inputs");
        for &(a, b, want) in rows {
            let got = method(f(a), f(b)).to_bits();
            assert_eq!(
                got,
                libm_binary(name)(f(a), f(b)).to_bits(),
                "{name}({a:#018x}, {b:#018x}) is libm's"
            );
            assert_eq!(
                got, want,
                "{name}({a:#018x}, {b:#018x}) is the Mac's libm bits"
            );
        }
    }
    for &(x, _) in UNARY[0].1 {
        let (s, c) = (m.sine_cosine)(f(x));
        assert_eq!(
            (s.to_bits(), c.to_bits()),
            (libm::sin(f(x)).to_bits(), libm::cos(f(x)).to_bits()),
            "sine_cosine"
        );
        if f(x) != 0.0 {
            let want = libm::sin(f(x)) / f(x);
            assert_eq!(
                (m.sinc)(f(x), f(x) * f(x)).to_bits(),
                want.to_bits(),
                "sinc({x:#018x})"
            );
        }
    }
}

#[test]
fn real_pinned_bits() {
    check_f64(&real::<f64>());
}

negative_control!(
    real_pinned_bits,
    "`sine` left on std, the platform's `sin`",
    expected = "sine(",
    {
        let mut m = real::<f64>();
        m.unary[0].1 = f64::sin;
        check_f64(&m)
    }
);

/// Checks `m`, the f32 methods, against `num_traits::Float`'s on the f64 table's inputs, narrowed.
fn check_f32(m: &Methods<f32>) {
    for ((name, method), (_, rows)) in m.unary.iter().zip(UNARY) {
        for x in rows
            .iter()
            .map(|&(x, _)| f64::from_bits(x) as f32)
            .chain([1e-10, -3e-8])
        {
            let (got, want) = (method(x), float_unary(name)(x));
            assert!(
                got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()),
                "{name}({x:e}) at f32"
            );
        }
    }
    let pairs = [(0.5f32, 2.0f32), (3.0, -4.0), (10.0, 3.0)];
    for (a, b) in pairs {
        assert_eq!(
            m.binary[1].1(a, b).to_bits(),
            Float::atan2(a, b).to_bits(),
            "arctangent2({a}, {b}) at f32"
        );
        assert_eq!(
            m.binary[2].1(a, b).to_bits(),
            Float::powf(a, b).to_bits(),
            "power({a}, {b}) at f32"
        );
        assert_eq!(
            m.binary[3].1(a, b).to_bits(),
            Float::hypot(a, b).to_bits(),
            "hypotenuse({a}, {b}) at f32"
        );
        let (s, c) = (m.sine_cosine)(a);
        assert_eq!(
            (s.to_bits(), c.to_bits()),
            (Float::sin(a).to_bits(), Float::cos(a).to_bits()),
            "sine_cosine"
        );
    }
}

#[test]
fn real_pinned_bits_f32() {
    check_f32(&real::<f32>());
}

negative_control!(
    real_pinned_bits_f32,
    "`exp_minus_one` as `exp(x) − 1`, which loses `x` near 0",
    expected = "exp_minus_one(",
    {
        let mut m = real::<f32>();
        m.unary[6].1 = |x: f32| Float::exp(x) - 1.0;
        check_f32(&m)
    }
);
