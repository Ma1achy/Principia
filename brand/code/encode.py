"""Turns a folder of transparent PNG frames (f00000.png ...) into MP4, WebM and GIF with ffmpeg.

The committed MP4s record libx264 crf 15 (x264 core 164, veryfast-class options) from ffmpeg 6.1;
the other settings were not recorded, so these are chosen (see README-build.md):
  MP4   frames composited on the page colour, libx264 -preset veryfast -crf 15, yuv420p
  WebM  libvpx-vp9 with an alpha plane (yuva420p), constant quality: -crf 28 -b:v 0
  GIF   composited on the page colour, scaled with Lanczos, one 256-colour palette for the whole
        loop (palettegen), Bayer dithering, looping forever
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
           f'[a]palettegen=max_colors=256[p];[b][p]paletteuse=dither=bayer:bayer_scale=5')
    _run(_inputs(frames, fps, w, h, BG[theme]) + ['-filter_complex', flt, '-loop', '0', out])
