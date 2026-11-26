// re-run each board and check: every trail piece lies wholly on one side of the P plane,
// and no trail point or body is inside the solid (outside the hole) within the slab
const fs=require('fs'); const g=JSON.parse(fs.readFileSync('pgeo.json'));
const inPoly=(p,poly)=>{let c=false;for(let i=0,j=poly.length-1;i<poly.length;j=i++){const a=poly[i],b=poly[j];if((a[1]>p[1])!==(b[1]>p[1])&&p[0]<(b[0]-a[0])*(p[1]-a[1])/(b[1]-a[1])+a[0])c=!c;}return c;};
for (const f of ['MothSolid','MothNight','Threaded','Slices','BurrauRing','WireButterfly','Orb_butterfly1','Orb_butterfly2','Orb_goggles','Orb_dragonfly','Dragonfly','DragonflyNight']) {
  const h=fs.readFileSync('p4/'+f+'.dc.html','utf8');
  let js=h.match(/data-dc-script[^>]*>([\s\S]*?)<\/script>/)[1];
  // expose orb() and front() for checking
  js=js.replace('const o = orb(t)','globalThis.__orb = orb; globalThis.__H = H; const o = orb(t)');
  const C=new Function('DCLogic',js+'; return Component;')(class{constructor(){this.props={};}});
  let solid=0, n=0, bad=0;
  for (let t=0;t<40;t+=0.25){ const c=new C(); c.state={t}; const v=c.renderVals();
    bad+=[...v.itemsBack,...v.itemsMid,...v.itemsB].filter(x=>/NaN|undefined/.test(JSON.stringify(x))).length;
    const o=__orb(t);
    const pts=[...o.trails.flatMap(tr=>tr.pts), ...o.bodies.map(b=>b.p)];
    for (const p of pts){ n++; if (Math.abs(p[2])<__H && inPoly(p,g.out) && !inPoly(p,g.hole)) solid++; }
  }
  console.log(f.padEnd(14),'points',n,'inside solid',solid,'bad',bad);
}
