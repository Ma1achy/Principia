#!/usr/bin/env python3
"""The N_sub threshold tables' reference (R-502; REQ-INT-091, REQ-INT-093): writes `tables.json` beside this script.

For each configuration of CONFIGS it evaluates THR[n] = (r_sub * n^(-1/gamma_sub))^2, n = 1 ... L_MAX, in Python's
`decimal` at 50 significant digits, and rounds each value to f32 (round to nearest, ties to even) straight from the
decimal value, through exact rational arithmetic, never through float() and f64. So the fixture is produced
independently of the host generator (`kernel::driver::nsub_table`), whose f64 evaluation rounds four times before its
one rounding to f32.

The inputs are the f64 values the generator receives: each configuration's r_sub and gamma_sub as written here, parsed
to the nearest binary64 (as Rust's and serde's parses do), then taken into `decimal` exactly. The formula is evaluated at
those inputs exactly; only the result is rounded.

Where the generator may legitimately round to the other side. The generator's f64 value differs from the exact one by
a few f64 ulps: e = -1/gamma_sub rounded (half an ulp, amplified by |e ln n| = ln n / gamma_sub in pow's result), pow's
own error (under one ulp), the product with r_sub (half an ulp), the square, which doubles each relative error, and its
own rounding (half an ulp): at most ln n / gamma_sub + 3.5 f64 ulps. An entry whose exact value lies within
BAND(n) = 4 (ln n / gamma_sub + 3) f64 ulps of an f32 rounding tie, more than twice that bound, may be rounded to
either neighbour by the generator, by a double rounding through f64 or the pinned libm's sub-ulp f64 error. Each such
entry is listed under the configuration's "exceptions" with that reason, its correctly rounded bits and the other
neighbour's; the test accepts either there, and checks the generator's bits against its own rounding of its f64 value.
Every other entry must equal the correctly rounded bits exactly.

Run: python3 fixtures/gates/nsub-table/reference.py
"""

import json
import math
import os
from decimal import Decimal, getcontext
from fractions import Fraction

getcontext().prec = 50

# The table's maximum length, the constants register's `nsub_table_max` (REQ-INT-094, provisional until the M3 gate).
L_MAX = 256

# (r_sub, gamma_sub, N_max): the default, then r_sub, gamma_sub and N_max each changed, N_max at a non-power of two and
# at L_MAX, and two with all three changed. N_max does not enter the thresholds; it is recorded for the refusal checks.
CONFIGS = [
    ("0.05", "1.5", 64),
    ("0.08", "1.5", 64),
    ("0.05", "2.0", 64),
    ("0.05", "1.5", 50),
    ("0.05", "1.5", 256),
    ("0.02", "1.0", 100),
    ("0.1", "3.0", 7),
]

# f32's significand bits after the leading one, and an f64 ulp in f32 ulps (for normal values).
F32_FRACTION = 23
F64_ULP_IN_F32_ULPS = Fraction(1, 2**29)


def exact_input(text):
    """The binary64 nearest `text`, taken exactly as a Decimal (the generator's input)."""
    return Decimal(float(text))


def threshold(r_sub, gamma_sub, n):
    """(r_sub * n^(-1/gamma_sub))^2 at 50 significant digits."""
    e = Decimal(-1) / gamma_sub
    s = r_sub * (Decimal(n) ** e)
    return s * s


def round_f32(value):
    """The f32 nearest the positive normal `value` (ties to even), from its exact rational value: its bits, the other
    neighbour's bits across the nearest rounding tie, and the distance from that tie in f32 ulps."""
    f = Fraction(value)
    assert f > 0
    e = f.numerator.bit_length() - f.denominator.bit_length()
    while Fraction(2) ** e > f:
        e -= 1
    while Fraction(2) ** (e + 1) <= f:
        e += 1
    assert -126 <= e <= 127, f"{value} is not a normal f32"
    scaled = f / Fraction(2) ** (e - F32_FRACTION)
    m = math.floor(scaled)
    rem = scaled - m
    up = rem > Fraction(1, 2) or (rem == Fraction(1, 2) and m % 2 == 1)
    bits_down = ((e + 127) << F32_FRACTION) | (m - 2**F32_FRACTION)
    bits = bits_down + 1 if up else bits_down
    other = bits_down if up else bits_down + 1
    return bits, other, abs(rem - Fraction(1, 2))


def band(n, gamma_sub):
    """BAND(n) in f32 ulps (the module's note)."""
    f64_ulps = 4 * (math.log(n) / float(gamma_sub) + 3)
    return Fraction(f64_ulps) * F64_ULP_IN_F32_ULPS


def table(r_text, gamma_text, n_max):
    r_sub, gamma_sub = exact_input(r_text), exact_input(gamma_text)
    bits, exceptions = [], []
    for n in range(1, L_MAX + 1):
        b, other, tie = round_f32(threshold(r_sub, gamma_sub, n))
        bits.append(f"0x{b:08X}")
        if tie < band(n, gamma_sub):
            exceptions.append(
                {
                    "n": n,
                    "reason": "within BAND(n) of an f32 rounding tie: a double rounding through f64 or the pinned "
                    "libm's sub-ulp f64 error may take the generator to the other neighbour",
                    "correctly_rounded": f"0x{b:08X}",
                    "other": f"0x{other:08X}",
                }
            )
    return {
        "r_sub": float(r_text),
        "gamma_sub": float(gamma_text),
        "n_max": n_max,
        "bits": bits,
        "exceptions": exceptions,
    }


def main():
    out = {
        "description": "THR[n] = round_f32((r_sub·n^(−1/γ_sub))²), n = 1 … l_max, correctly rounded from 50-digit "
        "decimal values by reference.py, for the host generator's test (REQ-INT-091) and the wasm host's "
        "(REQ-INT-093)",
        "l_max": L_MAX,
        "configs": [table(*c) for c in CONFIGS],
    }
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "tables.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(out, f, indent=1, ensure_ascii=False)
        f.write("\n")
    listed = sum(len(c["exceptions"]) for c in out["configs"])
    print(f"wrote {path}: {len(CONFIGS)} configurations, {L_MAX} entries each, {listed} listed exceptions")


if __name__ == "__main__":
    main()
