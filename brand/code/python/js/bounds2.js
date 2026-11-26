// extent from the frame centre (320,320) of trails AND glowing bodies (centre ± visible glow radius)
const fs=require('fs'); const f=process.argv[2];
const h=fs.readFileSync('p4/'+f+'.dc.html','utf8');
let js=h.match(/data-dc-script[^>]*>([\s\S]*?)<\/script>/)[1];
const C=new Function('DCLogic',js+'; return Component;')(class{constructor(){this.props={};}});
let L=0,R=0,U=0,D=0;
for (let t=0;t<40;t+=0.1){ const c=new C(); c.state={t}; const v=c.renderVals();
  for (const it of [...v.itemsBack,...v.itemsMid,...v.itemsB]) {
    if (it.f && it.f.startsWith('url(#e')) { const m=it.d.match(/^M(-?[\d.]+),(-?[\d.]+)a(-?[\d.]+)/); const r=+m[3]*0.55, cx=+m[1]+(+m[3]), cy=+m[2];
      L=Math.max(L,320-(cx-r)); R=Math.max(R,(cx+r)-320); U=Math.max(U,320-(cy-r)); D=Math.max(D,(cy+r)-320); continue; }
    if (it.k<=-1e3 || it.d.includes('a')) continue;
    for (const m of it.d.matchAll(/(-?[\d.]+),(-?[\d.]+)/g)) { const x=+m[1], y=+m[2]; L=Math.max(L,320-x); R=Math.max(R,x-320); U=Math.max(U,320-y); D=Math.max(D,y-320); } } }
console.log(JSON.stringify({L,R,U,D}));
