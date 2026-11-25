# Social preview

`make_social_preview.py` renders the 1280 × 640 social-preview image in headless Chrome. It uses the README hero banner (`docs/readme/hero.jpg`) as the background, tinted dark or light, with a still frame of the animated 2D title and its "Planar Three-Body Problem" subtitle (`brand/code/web/`) centred on top.

```
pip install playwright pillow
python -m playwright install chromium
python make_social_preview.py              # writes out/social-preview-{dark,light}.{png,jpg}
python make_social_preview.py --frame 20   # freeze a different moment of the 36 s loop
```

To use it, open the repo's Settings → General → Social preview → Edit, and upload a `.jpg` from `out/`. GitHub needs the file to be under 1 MB; each JPG here is about 100 KB.

`brand/code/build.py --part social` runs this script with `--web`, `--out` and `--hero` pointing into the rebuild folder, so nothing here is overwritten; `python3 brand/code/compare.py brand <rebuild>` then compares the PNGs and JPGs with these.
