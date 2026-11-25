import re, json, os
OUT='/home/claude/pack/principia-brand/code/web'
os.makedirs(OUT,exist_ok=True)
VARIANTS={'icon-light':'Dragonfly','icon-dark':'DragonflyNight','title-2d-light':'WordFlat','title-2d-dark':'WordFlatNight','title-2d-subtitle-light':'WordFlatSub','title-2d-subtitle-dark':'WordFlatSubNight'}
RUNTIME=r'''
  class DCLogic { constructor(props) { this.props = props || {}; this.state = {}; } setState(s) { Object.assign(this.state, s); if (this.__r) this.__r(); } }
  function expand(tpl, v) {
    let out = tpl.replace(/<sc-for list="\{\{(\w+)\}\}" as="(\w+)"[^>]*>([\s\S]*?)<\/sc-for>/g, (m, L, as, inner) =>
      (v[L] || []).map(x => inner.replace(new RegExp('\\{\\{' + as + '\\.(\\w+)\\}\\}', 'g'), (mm, k) => x[k])).join(''));
    return out.replace(/\{\{(\w+)\}\}/g, (m, k) => v[k]);
  }
'''
for name,board in VARIANTS.items():
    h=open(f'p4/{board}.dc.html').read()
    svg=re.search(r'<svg[\s\S]*?</svg>',h).group(0)
    root=re.search(r'<x-dc>[\s\S]*?(<div style="[^"]*">)\s*<svg',h).group(1)
    js=re.search(r'data-dc-script[^>]*>([\s\S]*?)</script>',h).group(1)
    props=json.loads(re.search(r"data-props='([^']*)'",h).group(1))
    defaults={k:v['default'] for k,v in props.items() if not k.startswith('$') and 'default' in v}
    W,H=props['$preview']['width'],props['$preview']['height']
    lib=f'''/* Principia mark: {name}. Self-contained: no dependencies.
   Usage: PrincipiaMark['{name}'].mount(element, {{ speed, glow, weight, size, still, frame, transparent, backdrop }})
          PrincipiaMark['{name}'].svgAt(t, props)  -> SVG string of the frame at time t (seconds)
   Generated from the three-body simulations in ../python. */
(function () {{
{RUNTIME}
  const TEMPLATE = {json.dumps(root+svg+'</div>')};
  const DEFAULTS = {json.dumps(defaults)};
  const SIZE = [{W}, {H}];
{js}
  function make(props) {{ return new Component(Object.assign({{}}, props)); }}
  function svgAt(t, props) {{ const c = make(Object.assign({{ still: true, frame: t }}, props)); return expand(TEMPLATE, c.renderVals()); }}
  function mount(el, props) {{
    const c = make(props);
    c.__r = () => {{ el.innerHTML = expand(TEMPLATE, c.renderVals()); }};
    c.__r(); if (c.componentDidMount) c.componentDidMount();
    return {{ destroy() {{ if (c.componentWillUnmount) c.componentWillUnmount(); }}, seek(t) {{ if (c.componentWillUnmount) c.componentWillUnmount(); c.props.still = true; c.props.frame = t; c.__r(); }} }};
  }}
  const g = typeof window !== 'undefined' ? window : globalThis;
  g.PrincipiaMark = g.PrincipiaMark || {{}};
  g.PrincipiaMark['{name}'] = {{ mount, svgAt, size: SIZE, defaults: DEFAULTS }};
}})();
'''
    open(f'{OUT}/principia-{name}.js','w').write(lib)
    dark='dark' in name
    bg='#0d0c0b' if dark else '#efe9dc'
    open(f'{OUT}/{name}.html','w').write(f'''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Principia · {name}</title>
<style>html,body{{margin:0;height:100%;background:{bg}}}body{{display:grid;place-items:center}}#m{{line-height:0}}</style></head>
<body><div id="m"></div>
<script src="principia-{name}.js"></script>
<script>
  // ?t=12 shows a still frame at 12 s; otherwise it animates
  const q = new URLSearchParams(location.search), t = q.get('t');
  const props = t !== null ? {{ still: true, frame: +t }} : {{}};
  if (q.get('transparent') === '0') props.transparent = false;
  if (q.get('size')) props.size = +q.get('size');
  if (q.get('weight')) props.weight = +q.get('weight');
  window.__mark = PrincipiaMark['{name}'].mount(document.getElementById('m'), props);
  window.__seek = (s) => window.__mark.seek(s);
</script></body></html>
''')
    print(name, len(lib))
# animated favicon
open(f'{OUT}/favicon-animated.js','w').write('''/* Animated Principia favicon (Chrome, Edge, Firefox; Safari keeps the static favicon).
   Include principia-icon-light.js and principia-icon-dark.js first, then this file.
   It follows prefers-color-scheme, redraws the tab icon ~15 times a second, and pauses when the tab is hidden. */
(function () {
  const FPS = 15, PX = 64;
  const link = document.querySelector('link[rel~="icon"]') || Object.assign(document.head.appendChild(document.createElement('link')), { rel: 'icon' });
  const cv = document.createElement('canvas'); cv.width = cv.height = PX; const cx = cv.getContext('2d');
  const dark = matchMedia('(prefers-color-scheme: dark)');
  const reduce = matchMedia('(prefers-reduced-motion: reduce)');
  let t0 = performance.now(), busy = false;
  function draw(t) {
    if (busy) return; busy = true;
    const mark = PrincipiaMark[dark.matches ? 'icon-dark' : 'icon-light'];
    const svg = mark.svgAt(t, { size: PX, weight: 1.6, transparent: true, backdrop: 'transparent' }).replace('<svg ', '<svg xmlns="http://www.w3.org/2000/svg" ');
    const img = new Image();
    img.onload = () => { cx.clearRect(0, 0, PX, PX); cx.drawImage(img, 0, 0, PX, PX); link.href = cv.toDataURL('image/png'); busy = false; };
    img.onerror = () => { busy = false; };
    img.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(svg.replace(/^<div[^>]*>/, '').replace(/<\\/div>$/, ''));
  }
  if (reduce.matches) { draw(17); return; }
  setInterval(() => { if (!document.hidden) draw(6 + (performance.now() - t0) / 1000); }, 1000 / FPS);
})();
''')
open(f'{OUT}/favicon-demo.html','w').write('''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Principia</title>
<link rel="icon" href="../../icons/web/favicon.ico" sizes="any">
<link rel="icon" href="../../icons/web/favicon.svg" type="image/svg+xml">
<script src="principia-icon-light.js"></script><script src="principia-icon-dark.js"></script>
<script src="favicon-animated.js" defer></script>
<style>body{font:16px/1.5 "Helvetica Neue",Helvetica,Arial,sans-serif;margin:48px;color:#1d1b19;background:#efe9dc}@media (prefers-color-scheme: dark){body{background:#0d0c0b;color:#efe9dc}}</style>
</head><body><p>Look at this tab: the favicon is animated.</p></body></html>
''')
