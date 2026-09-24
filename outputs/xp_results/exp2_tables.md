probes: 40 attempted, 23 usable, 17 failed
  FAILED far r~10 t=30.0 half=0.05: timeout
  FAILED far r~10 t=30.0 half=0.025: timeout
  FAILED body1 slice t=30.0 half=0.05: timeout
  FAILED near-field t=60.0 half=0.025: timeout
  FAILED far r~10 t=60.0 half=0.05: timeout
  FAILED far r~10 t=60.0 half=0.025: timeout
  FAILED body2 mid t=60.0 half=0.05: timeout
  FAILED body1 slice t=60.0 half=0.05: timeout
  FAILED body1 slice t=60.0 half=0.025: timeout
  FAILED near-field t=120.0 half=0.05: timeout
  FAILED near-field t=120.0 half=0.025: timeout
  FAILED body2 mid t=120.0 half=0.05: timeout
  FAILED body2 mid t=120.0 half=0.025: timeout
  FAILED far r~10 t=120.0 half=0.05: timeout
  FAILED far r~10 t=120.0 half=0.025: timeout
  FAILED body1 slice t=120.0 half=0.05: timeout
  FAILED body1 slice t=120.0 half=0.025: timeout

## Escape fraction vs horizon (parent quad half=0.05, gate 1e-3)

| region | t=13 | t=30 | t=60 | t=120 | retained@t=120 | med drift @t=120 |
|---|---|---|---|---|---|---|
| near-field | 0.00 | 0.01 | 0.56 | - | - | - |
| mid-field | 1.00 | 1.00 | 1.00 | 1.00 | 0.70 | 7.9e-05 |
| far r~10 | 0.00 | - | - | - | - | - |
| body2 mid | 0.98 | 1.00 | - | - | - | - |
| body1 slice | 0.00 | - | - | - | - | - |

## t_end exponent, parent (half=0.05) vs child (half=0.025), escaped copies only

| region | t | esc frac | fp parent | fp child | t_end spread P | t_end spread C | **alpha t_end** | alpha_E control | trust |
|---|---|---|---|---|---|---|---|---|---|
| mid-field | 13 | 1.00 | 16/16 | 16/16 | 1.0994e-02 | 0.0000e+00 | **nan** | 1.0251 | ok |
| mid-field | 30 | 1.00 | 16/16 | 16/16 | 2.2204e-16 | 0.0000e+00 | **nan** | 0.9722 | ok |
| mid-field | 60 | 1.00 | 16/16 | 16/16 | 6.6613e-16 | 1.0547e-15 | **nan** | 0.8148 | **UNTRUSTWORTHY** |
| mid-field | 120 | 1.00 | 16/16 | 15/16 | 7.2164e-16 | 7.6975e-16 | **nan** | 0.7860 | **UNTRUSTWORTHY** |
| body2 mid | 13 | 0.98 | 16/16 | 16/16 | 0.0000e+00 | 0.0000e+00 | **nan** | 0.9628 | ok |
| body2 mid | 30 | 1.00 | 16/16 | 16/16 | 0.0000e+00 | 0.0000e+00 | **nan** | 1.0171 | ok |
