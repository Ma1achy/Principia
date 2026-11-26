"""Turns a folder of transparent PNG frames (f00000.png ...) into MP4, WebM and GIF with ffmpeg.

The committed MP4s record libx264 crf 15 (x264 core 164, veryfast-class options) from ffmpeg 6.1;
the other settings were not recorded, so these are chosen (see README-build.md):
  MP4   frames composited on the page colour, libx264 -preset veryfast -crf 15, yuv420p
  WebM  libvpx-vp9 with an alpha plane (yuva420p), constant quality: -crf 28 -b:v 0
  GIF   composited on the page colour, scaled with Lanczos, a 256-colour palette per frame
        (palettegen stats_mode=single, paletteuse new=1; of the settings tried it came closest to the
        committed GIFs), looping forever
  WebP  animated, lossy, with an alpha plane, scaled with Lanczos, looping forever; quality is set per file
        by webp() to the highest that fits the size limit
"""
import subprocess

BG = {'light': '0xefe9dc', 'dark': '0x0d0c0b'}


def _run(cmd):
    subprocess.run(['ffmpeg', '-v', 'error', '-nostdin', '-y'] + cmd, check=True)


def _inputs(frames, fps, w, h, bg):
    return ['-framerate', str(fps), '-i', f'{frames}/f%05d.png', '-f', 'lavfi', '-i', f'color=c={bg}:s={w}x{h}:r={fps}']


def mp4(frames, fps, w, h, theme, out):
    _run(_inputs(frames, fps, w, h, BG[theme]) + ['-filter_complex', '[1][0]overlay=shortest=1,format=yuv420p',
         '-c:v', 'libx264', '-preset', 'veryfast', '-crf', '15', '-pix_fmt', 'yuv420p', out])


def webm(frames, fps, out):
    _run(['-framerate', str(fps), '-i', f'{frames}/f%05d.png', '-c:v', 'libvpx-vp9', '-pix_fmt', 'yuva420p',
          '-crf', '28', '-b:v', '0', '-auto-alt-ref', '0', '-row-mt', '1', out])


def gif(frames, fps, w, h, theme, gif_fps, gif_w, out):
    flt = (f'[1][0]overlay=shortest=1,fps={gif_fps},scale={gif_w}:-1:flags=lanczos,split[a][b];'
           f'[a]palettegen=stats_mode=single[p];[b][p]paletteuse=new=1')
    _run(_inputs(frames, fps, w, h, BG[theme]) + ['-filter_complex', flt, '-loop', '0', out])


def webp(frames, fps, out_fps, width, out, limit=None, quality=None, border=None):
    """Animated lossy WebP written with Pillow (ffmpeg's libwebp_anim cannot lower the alpha quality, and
    losslessly kept alpha alone is more than 4 MB). Colour and alpha share one quality number. With limit
    (bytes) the highest quality (1 to 100) whose file fits is used; the result is (quality, bytes).
    Frame durations are 67, 67, 66 ms so that 15 fps comes to exactly 36 s over 540 frames."""
    import os
    from PIL import Image
    step = int(round(fps / out_fps))
    names = sorted(os.listdir(frames))[::step]
    ims = []
    for n in names:
        im = Image.open(f'{frames}/{n}').convert('RGBa')           # premultiplied: no dark fringes when scaled
        h = round(im.height * width / im.width)
        ims.append(im.resize((width, h), Image.LANCZOS).convert('RGBA'))
    dur = [67, 67, 66] * (len(ims) // 3 + 1)
    def write(q):
        ims[0].save(out, save_all=True, append_images=ims[1:], duration=dur[:len(ims)], loop=0, lossless=False,
                    quality=q, alpha_quality=q, method=4)
        return os.path.getsize(out)
    if quality is not None:
        return quality, write(quality)
    lo, hi, best = 1, 100, None                         # file size grows with quality: bisect for the highest that fits
    while lo <= hi:
        q = (lo + hi) // 2
        size = write(q)
        if limit is None or size <= limit:
            best, lo = (q, size), q + 1
        else:
            hi = q - 1
    if best is None:
        return 1, write(1)
    q = best[0]
    if border:                       # lossy colour can drift by a level or two: go down until the border is within tolerance
        import compare
        colour, tol = border
        while q > 1:
            size = write(q)
            worst, _ = compare.border_check(compare.webp_frames(out), colour, tol)
            if worst <= tol:
                return q, size
            q -= 1
    return q, write(q)
