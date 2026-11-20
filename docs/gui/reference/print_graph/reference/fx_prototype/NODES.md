# Node catalogue

61 nodes.

| node | inputs | outputs | defaults | what it does |
|---|---|---|---|---|
| `expr` | — | out | f=0 | Arithmetic glue: evaluate `f` over the inputs (pixels arrive as floats). H, W, yy, xx are defined. |
| `u8` | x | out | — | Print to 8-bit pixels (truncating, as the media always have). |
| `rng` | — | rng | seed=0 |  |
| `draw` | rng | out, rng | kind=random, shape=HW, a=0.0, b=1.0 | Draw from a random stream. shape: 'HW', 'HW1', 'N1' (N from input n), or a list. |
| `flow` | image | tx, ty, coh | sigma=4.0 | Unit tangents along the image's contours (structure tensor), and how coherent they are. |
| `field_flow` | image, E | tx, ty | — | The fine flow where the filaments are fine, the broad flow where the field is calm. |
| `edge_density` | image | out | s=8 | How fast the picture changes here: the density of fine fringes. |
| `regions` | image | labels, palette | K=8, b=1.5, mode=5 | Flat colour regions: blur, median-cut to K colours, mode-filter. |
| `quantize` | image | labels, palette | K=26 | Median-cut to K colours, no smoothing. |
| `mode_filter` | labels | out | size=7 |  |
| `boundary` | labels | out | — |  |
| `distance` | mask | out | — |  |
| `morph` | mask | out | op=dilation, iterations=1 |  |
| `relabel` | labels | labels, n | — |  |
| `mean_by_label` | image, labels | out | n=None |  |
| `pil_gray` | image | out | — |  |
| `noise` | — | out | scale=10.0, octaves=4, seed=0, h=None, w=None | Fractal noise, zero mean and unit spread. |
| `paper_tooth` | — | out | — | The scanned cold-press paper, tiled. |
| `paper_ground` | — | out | seed=0 | The poster's cream stock, with its tooth. |
| `canvas` | — | out | period=5.0, seed=0, scale=1 | Canvas weave. |
| `grain` | — | out | seed=0, size=1.0 | Film grain, unit spread. |
| `jitter_points` | rng | points, rng | spacing=8.0, jitter=0.45 | A jittered grid of points: even, never regular. |
| `shuffle` | points, rng | points, rng | — |  |
| `voronoi` | points | out | — |  |
| `streamlines` | tx, ty, density | out | dmin=2.0, dmax=10.0, tmin=0.0, seed=0, maxlen=500, minlen=8 | Evenly spaced streamlines along the flow, closer where `density` is high. |
| `bayer` | like | out | n=4 | An ordered-dither threshold map the size of `like`. |
| `smear` | x, tx, ty | out | length=6, step=1.0, taper=True | Drag values along the flow (line-integral convolution). |
| `strokes` | tx, ty | out | width=2.0, density=0.005, length=16, seed=0, amp_var=0.6 | A field of tapered strokes along the flow: sparse dabs, `width` across, dragged `length` each way. |
| `kuwahara` | image | out | r=4 |  |
| `light` | height | diff, spec | strength=1.0, L=[-0.55, -0.75, 0.9], shin=24 | Light a height field from the upper left: diffuse (1 on flat ground) and specular. |
| `cast_shadow` | height | out | L=[-0.55, -0.75], reach=6, k=0.9 |  |
| `shift` | x | out | dy=0.0, dx=0.0, order=1 |  |
| `warp` | x, dy, dx | out | order=0 | Resample x at (y+dy, x+dx). |
| `film_glow` | e | out | bloom=1.0, halation=1.0, bleed=1.0 | What film does to bright light: bloom, red-orange halation, and a wide bleed. |
| `separate_cmyk` | image | c, m, y, k | ucr=0.8, rich=0.55 | CMYK with grey replacement and a rich-black underlay. |
| `palette_mode` | image | out | mode=wheel | A palette mode: screen, wheel, limited, earth, nocturne, fluoro, duotone, inverse. |
| `pixelate_down` | image | out, gh, gw | size=3, rounding=ceil, gray=False | Average the image into cells of `size` pixels (a box filter). |
| `pixelate_up` | small | out | size=3 |  |
| `mediancut` | image | labels, palette | K=12, mode=3 |  |
| `fftblur` | x | out | s=1.0 |  |
| `to_pigment` | image | out | amount=0.7, enabled=True | Pull electric screen colours towards real pigments of the same hue. |
| `count_colours` | image | out | cap=12 | How many distinct colours the image has (capped). |
| `threshold_map` | like, rng | out, rng | kind=8x8 | A dither threshold per cell: Bayer 2x2, 4x4, 8x8, or random. |
| `pool` | labels, E | out | strength=0.4, reach=3.5 | Pigment pooling at the edge of every wash: denser paint near each region's border. |
| `pencil_strokes` | x | out | ks=7, ndir=8, boost=1.0 | Lu, Xu & Jia (2012): each edge drawn as a short line along its own direction. |
| `pencil_texture` | — | out | angle=45.0, length=26, seed=0 | Graphite hatching texture: noise dragged along one direction. |
| `tone` | P, J | out | — |  |
| `wcp_paper` | — | out | — |  |
| `wcp_texture` | — | out | — |  |
| `stamp_dots` | points, rng | out, rng | SS=3, rad=[0.6, 1.05], ecc=[0.85, 1.15], ink=[0.75, 1.0] | Hand-drawn dots: each a slightly different ellipse of ink (drawn supersampled). |
| `keep_points` | points, rng, p | points, rng | — | Keep each point with probability p (a field sampled at the point). |
| `impact_dots` | tone, rng | out, rng | pitch=3.2, bands=1.0, SS=3 | A 9-pin print head: bands eight dots tall, ordered dither, ribbon wearing down each band, each pass landing slightly off the last. `bands` scales the banding (0 = none). |
| `stones` | palette, rng | mask, colour, rng | rate=0.0006, rad=[1.5, 5] | Marbling 'stones': drops of ink in the palette's colours, scattered last. |
| `stitches` | points, rng, tx, ty, labels, palette, base | out, rng | SS=2, length=[5, 9], light=[-0.6, -0.8] | Satin stitches along the flow, each a raised twist of thread with a direction-dependent sheen. |
| `pen_draw` | image, lines_a, lines_b | out | paper=[0.97, 0.96, 0.93], mono=None, SS=3 | Draw polylines with fineliners: each line in the pen nearest the image's colour along it. |
| `mono_tone` | image, L | out | — | The pen plotter's density: darkness for monochrome renders, darkness plus colour otherwise. |
| `trails` | lines, rng, colour, weight | acc, rng | amp=[0.5, 1.0], speed=[2, 9], swing=0.25, floor=0.25, scale=0.5 | Light trails: each streamline exposed as a line of light, its brightness wavering with the hand's speed; light accumulates where trails cross. |
| `trails_solid` | lines, rng, colour, weight, E | core, rng | dmin=2.2, dmax=12.0, widths=[0.8, 1.4, 2.4, 3.8, 6.0] | Light trails packed edge to edge, each as wide as half the gap to its neighbours, so the field is lit without gaps; thin in the filaments, broad where calm. |
| `tiles_along` | lines, rng | points, rng | s=11.0 | Set tile centres along each row, a tile's length apart (give or take). |
| `smalti` | colours, rng | colours, rng | K=28, vary=0.05 | Snap a set of colours to K glass colours, each piece varying slightly. |
| `scratches` | image, rng | out, rng | count=[1, 4], depth=0.25, width=0.6 | Fine vertical scratches along a strip of film. |
