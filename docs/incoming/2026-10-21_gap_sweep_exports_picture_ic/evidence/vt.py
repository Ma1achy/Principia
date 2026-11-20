# Can a time-sweep recording carry its spec? Two routes:
#  A. the PNG route: the spec in each frame's low bit, through GIF and lossy/lossless video
#  B. container metadata: GIF comment / application extension, MP4 udta comment, WebM/MKV tag and attachment
# For each: write, read back, compare bytes; then pass through a remux and a re-encode with ffmpeg defaults.
import os, subprocess, zlib, base64, hashlib, json, numpy as np
from PIL import Image
rng = np.random.default_rng(7)
W = H = 256; NF = 12
def run(*a): return subprocess.run(a, capture_output=True, text=True)
# frames: a smooth field with fine structure, like a slice
y, x = np.mgrid[0:H, 0:W] / W
frames = []
for f in range(NF):
    t = f / NF
    r = 0.5 + 0.5 * np.sin(14 * x + 6 * np.sin(9 * y + t * 6))
    g = 0.5 + 0.5 * np.sin(11 * y - 5 * np.cos(7 * x - t * 6))
    b = 0.5 + 0.5 * np.sin(20 * (x - 0.5) * (y - 0.5) * 9 + t * 3)
    frames.append((np.stack([r, g, b], -1) * 255).astype(np.uint8))
results = {}
def payload(n): return rng.integers(0, 256, n, dtype=np.uint8).tobytes()
P = {'1.5KB': payload(1536), '6KB': payload(6144), '64KB': payload(65536)}

# ---------- A. low-bit route ----------
def embed(img, data):
    bits = np.unpackbits(np.frombuffer(data, np.uint8))
    flat = img.reshape(-1).copy()
    assert len(bits) <= flat.size
    flat[:len(bits)] = (flat[:len(bits)] & 0xFE) | bits
    return flat.reshape(img.shape)
def extract(img, n):
    return np.packbits(img.reshape(-1)[:n * 8] & 1).tobytes()
data = P['6KB']
emb = [embed(fr, data) for fr in frames]
os.makedirs('f', exist_ok=True)
for i, fr in enumerate(emb): Image.fromarray(fr).save(f'f/{i:03d}.png')
def ber(dec):  # bit error rate of the payload, worst frame
    worst = 0
    for i, fr in enumerate(dec[:NF]):
        got = np.unpackbits(np.frombuffer(extract(fr, len(data)), np.uint8))
        ref = np.unpackbits(np.frombuffer(data, np.uint8))
        worst = max(worst, float((got != ref).mean()))
    return worst
def read_video(path):
    import imageio.v3 as iio
    return [np.asarray(fr)[:, :, :3] for fr in iio.imiter(path, plugin='pyav') ] if False else [np.asarray(fr)[:, :, :3] for fr in __import__('imageio').v2.mimread(path, memtest=False)]
encs = {
 'png frames (control)': None,
 'gif ffmpeg palettegen': ['-filter_complex', '[0:v]split[a][b];[a]palettegen=max_colors=256[p];[b][p]paletteuse=dither=none', 'out.gif'],
 'gif PIL adaptive': 'pil',
 'mp4 h264 crf18 yuv420': ['-c:v', 'libx264', '-crf', '18', '-pix_fmt', 'yuv420p', 'out1.mp4'],
 'mp4 h264 crf0 yuv444 (lossless luma path)': ['-c:v', 'libx264', '-crf', '0', '-pix_fmt', 'yuv444p', 'out2.mp4'],
 'mp4 libx264rgb crf0 (lossless RGB)': ['-c:v', 'libx264rgb', '-crf', '0', 'out3.mp4'],
 'webm vp9 default': ['-c:v', 'libvpx-vp9', '-b:v', '0', '-crf', '30', 'out4.webm'],
 'webm vp9 lossless': ['-c:v', 'libvpx-vp9', '-lossless', '1', 'out5.webm'],
}
A = {}
for name, args in encs.items():
    if args is None:
        dec = [np.asarray(Image.open(f'f/{i:03d}.png').convert('RGB')) for i in range(NF)]
        A[name] = ber(dec); continue
    if args == 'pil':
        ims = [Image.fromarray(fr).quantize(256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE) for fr in emb]
        ims[0].save('out0.gif', save_all=True, append_images=ims[1:], duration=80, loop=0)
        fr = []; im = Image.open('out0.gif')
        for k in range(NF): im.seek(k); fr.append(np.asarray(im.convert('RGB')))
        A[name] = ber(fr); continue
    r = run('ffmpeg', '-y', '-loglevel', 'error', '-framerate', '12', '-i', 'f/%03d.png', *args)
    if r.returncode: A[name] = 'encode failed: ' + r.stderr.strip()[:120]; continue
    out = args[-1]
    r = run('ffmpeg', '-y', '-loglevel', 'error', '-i', out, '-pix_fmt', 'rgb24', f'dec_{out}_%03d.png')
    dec = [np.asarray(Image.open(f'dec_{out}_{i+1:03d}.png').convert('RGB')) for i in range(NF)]
    A[name] = ber(dec)
results['A_lowbit_worst_frame_bit_error_rate (0 = intact, 0.5 = destroyed)'] = A

# ---------- B. container metadata ----------
B = {}
spec = P['6KB']; b64 = base64.b64encode(zlib.compress(spec)).decode()
h = hashlib.sha256(spec).hexdigest()
open('spec.bin', 'wb').write(spec)
plain = [Image.fromarray(fr) for fr in frames]
Image.fromarray(frames[0]).save('p0.png')
# GIF comment extension (PIL writes the comment block)
pal = [im.quantize(256) for im in plain]
pal[0].save('c.gif', save_all=True, append_images=pal[1:], duration=80, loop=0, comment=b64.encode())
got = Image.open('c.gif').info.get('comment', b'')
B['gif comment ext (PIL write/read)'] = 'intact' if got == b64.encode() else f'lost ({len(got)} B back)'
def ffmeta(path):
    r = run('ffprobe', '-v', 'error', '-show_entries', 'format_tags:stream_tags', '-of', 'json', path)
    return json.loads(r.stdout or '{}')
def find_tag(j, key):
    for sect in [j.get('format', {})] + j.get('streams', []):
        for k, v in sect.get('tags', {}).items():
            if k.lower() == key.lower(): return v
    return None
def check(path, key):
    v = find_tag(ffmeta(path), key)
    return 'intact' if v == b64 else ('absent' if v is None else f'altered ({len(v)} vs {len(b64)} chars)')
# make base videos with metadata
run('ffmpeg', '-y', '-loglevel', 'error', '-framerate', '12', '-i', 'f/%03d.png', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-metadata', f'comment={b64}', 'm.mp4')
run('ffmpeg', '-y', '-loglevel', 'error', '-framerate', '12', '-i', 'f/%03d.png', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-movflags', 'use_metadata_tags', '-metadata', f'principia_spec={b64}', 'm2.mp4')
run('ffmpeg', '-y', '-loglevel', 'error', '-framerate', '12', '-i', 'f/%03d.png', '-c:v', 'libvpx-vp9', '-b:v', '0', '-crf', '30', '-metadata', f'PRINCIPIA_SPEC={b64}', 'm.webm')
r = run('ffmpeg', '-y', '-loglevel', 'error', '-framerate', '12', '-i', 'f/%03d.png', '-attach', 'spec.bin', '-metadata:s:t', 'mimetype=application/octet-stream', '-c:v', 'libvpx-vp9', '-b:v', '0', '-crf', '30', 'a.webm')
B['webm attachment (write)'] = 'refused by the WebM muxer: ' + r.stderr.strip().splitlines()[-1][:90] if r.returncode else 'written'
run('ffmpeg', '-y', '-loglevel', 'error', '-framerate', '12', '-i', 'f/%03d.png', '-attach', 'spec.bin', '-metadata:s:t', 'mimetype=application/octet-stream', '-c:v', 'libvpx-vp9', '-b:v', '0', '-crf', '30', 'a.mkv')
B['mp4 udta comment'] = check('m.mp4', 'comment')
B['mp4 custom key (use_metadata_tags)'] = check('m2.mp4', 'principia_spec')
B['webm tag PRINCIPIA_SPEC'] = check('m.webm', 'PRINCIPIA_SPEC')
r = run('ffmpeg', '-y', '-loglevel', 'error', '-dump_attachment:t:0', 'back.bin', '-i', 'a.mkv'); 
B['mkv attachment (exact bytes)'] = 'intact' if os.path.exists('back.bin') and open('back.bin','rb').read() == spec else 'lost'
# survival: remux (-c copy) and re-encode with ffmpeg defaults
for src, key, ext in [('m.mp4', 'comment', 'mp4'), ('m2.mp4', 'principia_spec', 'mp4'), ('m.webm', 'PRINCIPIA_SPEC', 'webm')]:
    run('ffmpeg', '-y', '-loglevel', 'error', '-i', src, '-c', 'copy', f'rm_{src}')
    run('ffmpeg', '-y', '-loglevel', 'error', '-i', src, f're_{src}')
    B[f'{src} {key}: after remux -c copy'] = check(f'rm_{src}', key)
    B[f'{src} {key}: after re-encode (defaults)'] = check(f're_{src}', key)
# gif through ffmpeg re-encode
run('ffmpeg', '-y', '-loglevel', 'error', '-i', 'c.gif', 're_c.gif')
got = Image.open('re_c.gif').info.get('comment', b'')
B['gif comment: after ffmpeg re-encode'] = 'intact' if got == b64.encode() else 'lost'
# size limits on the mp4 comment: 64 KB payload
big = base64.b64encode(zlib.compress(P['64KB'])).decode()
run('ffmpeg', '-y', '-loglevel', 'error', '-framerate', '12', '-i', 'f/%03d.png', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-metadata', f'comment={big}', 'big.mp4')
v = find_tag(ffmeta('big.mp4'), 'comment')
B['mp4 comment with a 64 KB spec'] = 'intact' if v == big else f'{"absent" if v is None else "truncated to %d chars of %d" % (len(v), len(big))}'
results['B_container_metadata'] = B
results['spec size used (deflated+base64)'] = f'{len(b64)} chars for 6 KB raw'
print(json.dumps(results, indent=1))
