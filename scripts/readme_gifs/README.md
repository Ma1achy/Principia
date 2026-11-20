# Principia explainer GIFs: generator

Three animations for the Principia README, each in a light (ink on paper) and a dark version. The committed GIFs live in `docs/readme/`; this folder regenerates them.

| Script | GIF | What it shows |
| --- | --- | --- |
| `fig_symmetry.py` | `symmetry-reduction` | 15 numbers down to the 8D manifold: centre-of-mass frame, rotation, mass, scale |
| `fig_regtime.py` | `regularised-time` | The same leapfrog integrator and step count, with a fixed step and a regularised step (dt = r dτ) |
| `fig_scaletime.py` | `scale-time` | Burrau's 3-4-5 problem as Sun/Earth/Moon, as everyday balls and with no units, in natural time units |

## Run it

```
pip install -r requirements.txt
python3 build_gifs.py            # all three, light and dark, into ../out/
python3 build_gifs.py symmetry   # one figure (symmetry | regtime | scaletime)
python3 build_gifs.py --preview  # a few still PNGs in ../out/preview/ instead of the GIFs
```

Output goes to `scripts/out/` (gitignored). To regenerate the README's copies in place, run with `PRINCIPIA_OUT=../../docs/readme`. A full run takes a few minutes. If `ffmpeg` is on your PATH, each GIF is re-encoded afterwards and ends up about a third of the size. Without it you still get working GIFs, just larger ones.

Dark files are named `NAME-dark.gif`. In the README they sit in a `<picture>` element so GitHub picks the right one.

Fonts: Liberation Sans and Liberation Mono if installed, otherwise Arial/Menlo/Courier New, otherwise DejaVu. The text will shift a little between fonts, so check a preview if you change machines.

## Where to change things

- **Palette, fonts, output folders:** `figkit.py`. Set `PRINCIPIA_THEME=dark` or `light` and `PRINCIPIA_OUT=/some/folder` as environment variables to run one script on its own, for example `PRINCIPIA_THEME=dark python3 fig_regtime.py preview`.
- **Symmetry figure:** the example system is `_P`, `_w`, `V0` at the top of `fig_symmetry.py`. The five stages and their dimension counts are `STAGES` and `COUNTS`. `MS_FRAMES` sets how many frames each stage takes.
- **Regularised-time figure:** `ECC` (eccentricity), `ORBITS`, and `N_REG` (the step budget) at the top of `fig_regtime.py`. Lower `N_REG` makes the fixed step fail faster.
- **Scale-time figure:** the problem is Burrau's (`MASS`, `X0`, `T_END`). The two physical scales are `TAU_STAR` (1 solar mass, 1 AU) and `TAU_BALL` (100 g, 10 cm). The artwork is `art()`, and the captions are in `CAPS`.
- **Speed and length:** `N_RUN`, `HOLD_START` and `HOLD_END` in `fig_scaletime.py`, and the `fps` argument of `save_gif` at the bottom of each script.

## Notes

- Burrau's problem is integrated once with SciPy's DOP853 at tight tolerances (energy drift about 3e-10) and sampled by dense output.
- The regularised-time demo is the two-body idea (dt = r dτ in a plain leapfrog), not the instrument's global regularisation.
- The Moon, Earth and Sun pictures are artwork. The labels give the 3 : 4 : 5 ratios the problem actually uses, and the footer says so.
