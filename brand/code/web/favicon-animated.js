/* Animated Principia favicon (Chrome, Edge, Firefox; Safari keeps the static favicon).
   Include principia-icon-light.js and principia-icon-dark.js first, then this file.
   It follows prefers-color-scheme, redraws the tab icon ~15 times a second, and pauses when the tab is hidden.  */
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
    img.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(svg.replace(/^<div[^>]*>/, '').replace(/<\/div>$/, ''));
  }
  if (reduce.matches) { draw(17); return; }
  setInterval(() => { if (!document.hidden) draw(6 + (performance.now() - t0) / 1000); }, 1000 / FPS);
})();
