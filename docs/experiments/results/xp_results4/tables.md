# Brief 4 -- raw tables

## Experiment 1 -- which wall binds

n = 120 probes. 5 jitter fractions [0.03125, 0.0625, 0.125, 0.25, 0.5] (16x span) at fixed half=0.05, 3 regions, 8 horizons, ens=7 (8 copies), eta=0.01. No gate; every footprint keeps all 8 copies.

### 1a. alpha_E, 16x lever -- and the aggregator decides it

`sigma_E` per footprint (MAD over all 8 copies), then combined over the 16 footprints. Left column combines by MEAN, right by MEDIAN. Nothing else differs.

| region | t=13 | t=20 | t=24 | t=27 | t=30 | t=33 | t=36 | t=40 |
|---|---|---|---|---|---|---|---|---|
| near-field (mean) | +1.003 | +0.986 | +0.986 | +1.000 | +0.991 | +0.990 | -1.595 | -1.778 |
| near-field (median) | +1.008 | +0.999 | +0.999 | +1.000 | +1.000 | +0.982 | +1.101 | +0.979 |
| mid-field (mean) | +1.001 | +0.999 | +0.999 | +1.009 | +1.000 | +1.030 | +0.339 | -2.294 |
| mid-field (median) | +1.001 | +1.005 | +1.004 | +1.010 | +1.008 | +0.965 | +1.004 | +1.005 |
| body2 core (mean) | +1.013 | -4.035 | -4.322 | -3.370 | -3.103 | +1.469 | -2.522 | -2.205 |
| body2 core (median) | +1.030 | +1.001 | +1.006 | +1.002 | +1.003 | +0.983 | +0.996 | +0.964 |

### 1b. error_ratio, median over footprints vs max over footprints

Both at jf=0.5. The median is the meter as specified; the max is the one Brief 3 recommended as a boolean flag.

| region | stat | t=13 | t=20 | t=24 | t=27 | t=30 | t=33 | t=36 | t=40 |
|---|---|---|---|---|---|---|---|---|---|
| near-field | median | 1 | 1 | 1 | 1 | 1 | 1 | 1.042 | 1.042 |
| near-field | max | 1.001 | 1 | 1 | 1 | 1.013 | 1 | 11.56 | 11.56 |
| mid-field | median | 1 | 0.9996 | 0.9996 | 0.9998 | 0.9997 | 0.9999 | 0.9997 | 1.014 |
| mid-field | max | 1.063 | 1.033 | 1.033 | 1.003 | 1.033 | 1.993 | 2.132 | 2.132 |
| body2 core | median | 1 | 1 | 1 | 1 | 1 | 1 | 1.08 | 1.08 |
| body2 core | max | 1.065 | 1.759 | 4.269 | 17.44 | 1561 | 4700 | 1561 | 1561 |

### 1c. Failure FRACTION -- the aggregator-free statistic

Fraction of the 16 footprints whose own `error_ratio` falls outside [1/1.05, 1.05]. This is what 1a and 1b are each summarising differently. If a horizon is a wall this jumps 0 -> 1 at one t; if it is a rate it climbs.


**near-field** (fraction of 16 footprints failing)

| jf | delta | t=13 | t=20 | t=24 | t=27 | t=30 | t=33 | t=36 | t=40 |
|---|---|---|---|---|---|---|---|---|---|
| 0.03125 | 1.042e-03 | 0.000 | 0.125 | 0.125 | 0.000 | 0.312 | 0.125 | 0.562 | 0.562 |
| 0.0625 | 2.083e-03 | 0.062 | 0.000 | 0.000 | 0.000 | 0.062 | 0.062 | 0.500 | 0.562 |
| 0.125 | 4.167e-03 | 0.062 | 0.000 | 0.000 | 0.062 | 0.062 | 0.125 | 0.312 | 0.438 |
| 0.25 | 8.333e-03 | 0.062 | 0.000 | 0.062 | 0.000 | 0.125 | 0.250 | 0.688 | 0.688 |
| 0.5 | 1.667e-02 | 0.062 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.500 | 0.562 |

**mid-field** (fraction of 16 footprints failing)

| jf | delta | t=13 | t=20 | t=24 | t=27 | t=30 | t=33 | t=36 | t=40 |
|---|---|---|---|---|---|---|---|---|---|
| 0.03125 | 1.042e-03 | 0.062 | 0.000 | 0.000 | 0.188 | 0.000 | 0.250 | 0.188 | 0.312 |
| 0.0625 | 2.083e-03 | 0.000 | 0.000 | 0.000 | 0.062 | 0.000 | 0.250 | 0.188 | 0.375 |
| 0.125 | 4.167e-03 | 0.062 | 0.000 | 0.000 | 0.062 | 0.000 | 0.250 | 0.188 | 0.375 |
| 0.25 | 8.333e-03 | 0.000 | 0.000 | 0.000 | 0.125 | 0.000 | 0.250 | 0.125 | 0.250 |
| 0.5 | 1.667e-02 | 0.062 | 0.000 | 0.000 | 0.062 | 0.000 | 0.188 | 0.250 | 0.438 |

**body2 core** (fraction of 16 footprints failing)

| jf | delta | t=13 | t=20 | t=24 | t=27 | t=30 | t=33 | t=36 | t=40 |
|---|---|---|---|---|---|---|---|---|---|
| 0.03125 | 1.042e-03 | 0.125 | 0.125 | 0.188 | 0.312 | 0.375 | 0.312 | 0.625 | 0.688 |
| 0.0625 | 2.083e-03 | 0.062 | 0.125 | 0.188 | 0.250 | 0.312 | 0.250 | 0.750 | 0.812 |
| 0.125 | 4.167e-03 | 0.000 | 0.188 | 0.250 | 0.188 | 0.375 | 0.312 | 0.812 | 0.812 |
| 0.25 | 8.333e-03 | 0.000 | 0.125 | 0.188 | 0.125 | 0.250 | 0.312 | 0.750 | 0.750 |
| 0.5 | 1.667e-02 | 0.062 | 0.188 | 0.250 | 0.312 | 0.250 | 0.375 | 0.562 | 0.562 |

### 1d. Does the failure fraction depend on jitter?

The discriminating test. Regress the failure fraction on ln(delta) and on t together, pooled over regions. A measurement horizon needs a NEGATIVE ln(delta) coefficient (bigger jitter fails sooner); a noise-floor horizon needs a POSITIVE one; representability needs ZERO.

| term | coefficient | std err | t-stat |
|---|---|---|---|
| ln(delta) | -0.00902 | 0.01511 | -0.60 |
| t | +0.01893 | 0.00180 | +10.53 |
| intercept | -0.36719 | 0.09790 | -3.75 |

n = 120 (region, jitter, horizon) cells.

Predicted shift across the 16x jitter span if the wall were jitter-set: ln(16)/lambda = 2.77 time units. Measured ln(delta) coefficient: -0.00902 per e-fold (std err 0.01511).


## Experiment 2 -- the float32 horizon

n = 135 points. `f32-IC` = float64 arithmetic with initial conditions rounded to float32 (representability alone). `f32-arith` = float32 throughout. `lf` = softened leapfrog, compared only to itself across precision.

Driver control: `xp_prec` at float64 reproduces `tb_az.integrate_az` bit-for-bit (max|dr| = 0.000e+00 at t=5 and t=13).


### body2 core -- alpha_E

| arm | t=1 | t=2 | t=5 | t=8 | t=10 | t=12 | t=15 | t=20 | t=25 | t\* |
|---|---|---|---|---|---|---|---|---|---|---|
| f64 | +0.999 | +0.999 | +1.000 | +0.999 | +1.009 | +1.009 | +1.034 | +0.941 | +0.979 | >25 |
| f32-IC | +0.999 | +0.999 | +1.000 | +0.999 | +1.009 | +1.009 | +1.034 | +0.941 | +0.979 | >25 |
| f32-arith | +0.086 | +1.786 | -0.212 | +1.893 | +1.853 | +1.868 | +1.722 | +1.891 | +1.308 | **1.0** |
| lf64 | +0.999 | +0.998 | +1.000 | +0.981 | +1.061 | +1.071 | +1.062 | +1.076 | +1.067 | **10.0** |
| lf32 | +0.999 | +1.013 | +1.005 | +0.924 | +0.909 | +1.086 | +0.914 | +1.204 | +0.928 | **8.0** |

### mid-field -- alpha_E

| arm | t=1 | t=2 | t=5 | t=8 | t=10 | t=12 | t=15 | t=20 | t=25 | t\* |
|---|---|---|---|---|---|---|---|---|---|---|
| f64 | +1.000 | +1.000 | +1.000 | +1.000 | +1.002 | +1.003 | +0.979 | +1.005 | +1.327 | **25.0** |
| f32-IC | +1.000 | +1.000 | +1.000 | +1.000 | +1.002 | +1.003 | +0.979 | +1.005 | +1.326 | **25.0** |
| f32-arith | +0.994 | +0.083 | -0.002 | +2.929 | +2.813 | +2.369 | -7.254 | +2.495 | -1.227 | **2.0** |
| lf64 | +1.000 | +0.948 | +0.948 | +0.948 | +0.947 | +0.957 | +0.874 | +0.970 | +0.953 | >25 |
| lf32 | +1.000 | +0.980 | +0.983 | +0.940 | +0.702 | +0.633 | +0.866 | +0.573 | +0.531 | **8.0** |

### near-field -- alpha_E

| arm | t=1 | t=2 | t=5 | t=8 | t=10 | t=12 | t=15 | t=20 | t=25 | t\* |
|---|---|---|---|---|---|---|---|---|---|---|
| f64 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +0.985 | >25 |
| f32-IC | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +0.984 | >25 |
| f32-arith | -0.028 | +0.065 | +0.002 | +0.141 | +0.105 | +0.226 | -0.132 | +0.305 | -0.025 | **1.0** |
| lf64 | +1.000 | +0.995 | +0.995 | +1.026 | +1.042 | +1.040 | +0.908 | +1.078 | +0.845 | **15.0** |
| lf32 | +1.000 | +1.034 | +1.011 | +0.898 | +0.945 | +0.891 | +0.772 | +0.762 | +0.604 | **8.0** |


## Experiment 3 -- is the escape statistic shadowing-robust?

n = 60 probes. Jitter ~1e-14 (round-off scale; measured sigma_E(0) = 3.95e-15), 5 seeds, 16 footprints x 8 copies = 128 trajectories each. No gate.

Copy 0 of every footprint is unjittered, hence bit-identical across seeds. It is EXCLUDED everywhere below -- including it would manufacture agreement that is not in the data.

Three columns, three different claims, deliberately not merged:

- **region fraction**: escapers / 112 over the whole region. The COARSE claim.

- **footprint fraction**: same fraction computed per footprint, then the across-seed spread averaged over footprints. The PER-PIXEL claim.

- **identity**: which body escapes. The FINE claim.


| region | t | region frac | across-seed sd | footprint-frac sd | identity agreement | majority identity stable |
|---|---|---|---|---|---|---|
| near-field | 20 | 0.0000 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| near-field | 40 | 0.1250 | 0.0000 | 0.0000 | 0.9821 | 0.94 |
| near-field | 80 | 0.5696 | 0.0131 | 0.0175 | 0.9643 | 1.00 |
| mid-field | 20 | 0.0000 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| mid-field | 40 | 0.1250 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| mid-field | 80 | 0.3625 | 0.0091 | 0.0162 | 0.9661 | 0.94 |
| body2 mid | 20 | 0.0000 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| body2 mid | 40 | 0.1875 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| body2 mid | 80 | 0.5000 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| body1 slice | 20 | 0.0000 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| body1 slice | 40 | 0.1250 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| body1 slice | 80 | 0.4929 | 0.0067 | 0.0164 | 0.9607 | 0.94 |


## Experiment 4 -- what a refinement level buys

n = 64 probes. Quad half-widths [0.1, 0.05, 0.025, 0.0125] (8x span) at fixed jf=0.5, so delta = jf*2*half/(N-1) moves with the quad instead of with the jitter. Experiment 1 moved the same delta the other way; the two overlap exactly at half=0.05, jf=0.5.

### 4a. alpha_E, mean- vs median-aggregated over footprints

Exponent from adjacent half-width pairs (2x lever each).

| region | agg | delta (geo-mean) | t=13 | t=20 | t=24 | t=27 | t=30 | t=33 | t=36 | t=40 |
|---|---|---|---|---|---|---|---|---|---|---|
| near-field | mean | 2.357e-02 | +1.074 | +1.145 | +1.345 | +1.286 | +1.492 | +2.043 | +2.133 | +2.674 |
| near-field | mean | 1.179e-02 | +0.994 | +1.004 | +0.990 | +0.999 | +0.986 | +0.941 | +1.539 | +1.534 |
| near-field | mean | 5.893e-03 | +1.000 | +0.995 | +1.007 | +0.970 | +0.975 | +1.019 | -3.520 | -3.510 |
| near-field | median | 2.357e-02 | +1.002 | +1.113 | +1.247 | +1.241 | +1.449 | +1.421 | +1.050 | +1.393 |
| near-field | median | 1.179e-02 | +1.001 | +1.001 | +1.001 | +1.001 | +0.974 | +0.909 | +1.218 | +1.185 |
| near-field | median | 5.893e-03 | +0.998 | +0.998 | +0.981 | +0.915 | +0.893 | +1.090 | +0.935 | +0.892 |
| mid-field | mean | 2.357e-02 | +0.988 | +0.999 | +0.999 | +1.088 | +1.083 | +1.124 | +1.053 | +0.929 |
| mid-field | mean | 1.179e-02 | +1.010 | +1.004 | +1.004 | +0.979 | +1.006 | +1.152 | +1.115 | +0.883 |
| mid-field | mean | 5.893e-03 | +1.001 | +1.000 | +1.000 | +0.998 | +1.001 | +0.958 | +1.000 | +1.279 |
| mid-field | median | 2.357e-02 | +1.009 | +0.985 | +0.985 | +1.055 | +1.021 | +1.287 | +1.076 | +0.871 |
| mid-field | median | 1.179e-02 | +1.000 | +1.027 | +1.027 | +1.003 | +1.040 | +1.041 | +1.069 | +1.010 |
| mid-field | median | 5.893e-03 | +1.001 | +0.996 | +0.996 | +0.987 | +1.004 | +1.002 | +1.004 | +1.222 |

### 4b. Failure fraction vs quad half-width


**near-field** (fraction of 16 footprints with error_ratio outside [1/1.05, 1.05])

| half | delta | t=13 | t=20 | t=24 | t=27 | t=30 | t=33 | t=36 | t=40 |
|---|---|---|---|---|---|---|---|---|---|
| 0.1 | 3.333e-02 | 0.062 | 0.250 | 0.375 | 0.312 | 0.500 | 0.500 | 0.812 | 0.812 |
| 0.05 | 1.667e-02 | 0.062 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.500 | 0.562 |
| 0.025 | 8.333e-03 | 0.000 | 0.062 | 0.125 | 0.000 | 0.188 | 0.062 | 0.438 | 0.500 |
| 0.0125 | 4.167e-03 | 0.000 | 0.000 | 0.000 | 0.250 | 0.250 | 0.062 | 0.688 | 0.812 |

**mid-field** (fraction of 16 footprints with error_ratio outside [1/1.05, 1.05])

| half | delta | t=13 | t=20 | t=24 | t=27 | t=30 | t=33 | t=36 | t=40 |
|---|---|---|---|---|---|---|---|---|---|
| 0.1 | 3.333e-02 | 0.000 | 0.000 | 0.000 | 0.125 | 0.125 | 0.375 | 0.188 | 0.188 |
| 0.05 | 1.667e-02 | 0.062 | 0.000 | 0.000 | 0.062 | 0.000 | 0.188 | 0.250 | 0.438 |
| 0.025 | 8.333e-03 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.062 | 0.000 | 0.188 |
| 0.0125 | 4.167e-03 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.125 |

### 4c. The budgeting rule

Same regression as 1d, but delta is moved by the quad size rather than the jitter. The brief's rule -- halving the spacing buys ln(2)/lambda = 0.693 time units -- requires the ln(delta) coefficient to be non-zero and negative.

| term | coefficient | std err | t-stat |
|---|---|---|---|
| ln(delta) | +0.07044 | 0.02955 | +2.38 |
| t | +0.01569 | 0.00278 | +5.64 |
| intercept | +0.04057 | 0.15414 | +0.26 |

n = 64 (region, half-width, horizon) cells. Failure rate in t: +0.01569 per time unit (std err 0.00278); a halving of delta shifts the fraction by -0.04883, equivalent to -3.112 time units of playhead.
