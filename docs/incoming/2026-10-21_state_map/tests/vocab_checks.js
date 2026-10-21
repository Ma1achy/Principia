const {chromium}=require('playwright');
(async()=>{const b=await chromium.launch();const errs=[];
const open=async f=>{const p=await b.newPage({viewport:{width:1400,height:1300}});p.on('pageerror',e=>errs.push(f+': '+e.message));await p.goto('file://'+process.cwd()+'/'+f);await p.waitForTimeout(600);return p;};
const setv=async(p,o)=>p.evaluate(o=>{for(const[k,v] of Object.entries(o)){$(k).value=v;$(k).dispatchEvent(new Event('change'));}},o);
const waitDone=async p=>{for(let i=0;i<1200;i++){const d=await p.evaluate(()=>SM.levels.length&&SM.levels[SM.levels.length-1].done);if(d)return i;await p.waitForTimeout(250);}return -1;};
// 1. Shape n̂ unchanged: sphere outputs, old build vs new, every sphere colour node and setting family
const sph=async p=>p.evaluate(()=>{const tc=CHARTS.find(c=>c.id==='B');const out=[];let s=12345;const rnd=()=>{s=(s*16807)%2147483647;return s/2147483647};
 const combos=[['colour'],['tiles','axes6','full','vmf','3'],['tiles','fib','golden','nearest','3'],['tiles','ico12','golden','topk','5'],['tiles','corner8','grad','vmf','8'],['bands']];
 for(const c of combos){$('sm_mode').value=c[0];if(c[1]){$('sm_gen').innerHTML=SM_GENS.sphere.map(([k])=>`<option value="${k}">${k}</option>`).join('');$('sm_gen').value=c[1];$('sm_swatch').value=c[2];$('sm_kernel').value=c[3];$('sm_kappa').value=c[4];}
  $('sm_target').value='B';
  for(let k=0;k<400;k++){const th=rnd()*6.283,ph=Math.acos(2*rnd()-1);const n=[Math.sin(ph)*Math.cos(th),Math.sin(ph)*Math.sin(th),Math.cos(ph)];out.push(...smOccupantLab(tc,{kind:'ok',u:th,v:ph,n}));}}
 return out;});
const pn=await open('state_flow.html'), po=await open('state_flow.v5.html');
await pn.evaluate(()=>select(CHARTS.find(c=>c.id==='B')));await po.evaluate(()=>select(CHARTS.find(c=>c.id==='B')));
const a=await sph(pn), o=await sph(po); let d=0,ne=0;for(let i=0;i<a.length;i++){const x=Math.abs(a[i]-o[i]);if(x>0)ne++;d=Math.max(d,x);}
console.log('Shape n̂ unchanged: values',a.length,'differing',ne,'max |Δ|',d);
await po.close();
// 2. gamut of Replace-L with chroma following L, over every flat colour node and every brightness level used
console.log('Replace-L gamut',await pn.evaluate(()=>{const lv=[0.18,0.3,0.5,0.78,0.84];let bad=0,n=0,worst=null;const f=(L,a,b)=>{for(const B of lv){const k=Math.min(1,B/0.75);n++;if(!inGamut(B,a*k,b*k)){bad++;worst=[B,a,b]}}};
 for(let i=0;i<=100;i++)for(let j=0;j<=100;j++){const x=-1+2*i/100,y=-1+2*j/100;f(0,SM_C.plane*x,SM_C.plane*y);if(x*x+y*y<=1)f(0,SM_C.cyl*x,SM_C.cyl*y);}
 for(const A of [-1,0,1])for(const B of [-1,0,1])f(0,SM_TC*A,SM_TC*B);return bad+'/'+n+' '+JSON.stringify(worst);}));
console.log('map gamut',await pn.evaluate(()=>{let bad=0,n=0;for(let i=0;i<=200;i++)for(let j=0;j<=200;j++){const x=-1+2*i/200,y=-1+2*j/200;n+=2;if(!inGamut(0.75,SM_C.plane*x,SM_C.plane*y))bad++;if(!inGamut(0.52,SM_C.plane*x*SM_C.layer,SM_C.plane*y*SM_C.layer))bad++;}return bad+'/'+n;}));
// 3. the plane map is affine in s and one-to-one: binned at 2e-3 over a 300² grid, worst same-bin distance in s
console.log('plane one-to-one',await pn.evaluate(()=>{const m=new Map();let worst=0;for(let i=0;i<300;i++)for(let j=0;j<300;j++){const x=-1+2*(i+.5)/300,y=-1+2*(j+.5)/300,L=[0.75,SM_C.plane*x,SM_C.plane*y],k=L.map(v=>Math.round(v/2e-3)).join(',');const q=m.get(k);if(q)worst=Math.max(worst,Math.hypot(q[0]-x,q[1]-y));else m.set(k,[x,y]);}return 'worst same-bin |Δs| '+worst.toFixed(4)+' (bin width in s: '+(2e-3/SM_C.plane).toFixed(4)+')';}));
// 4. cylinder seam: tile and colour at θ = 0 and θ = 2π − ε agree (same tile), every pattern and n
console.log('cylinder seam',await pn.evaluate(()=>{const tc=CHARTS.find(c=>c.id==='G');$('sm_target').value='G';let bad=0,n=0;
 for(const pat of ['square','hex','tri','brick','sx','sy'])for(const N of ['4','6','8','12','18','24']){$('sm_tpat').innerHTML=SM_TPAT.cylinder.map(([k])=>`<option value="${k}">${k}</option>`).join('');$('sm_tpat').value=pat;$('sm_tn').value=N;
  for(let k=0;k<200;k++){const v=-0.6+1.2*Math.random(),A=smTileLab(tc,{kind:'ok',u:1e-9,v}),B=smTileLab(tc,{kind:'ok',u:2*Math.PI-1e-9,v}),C=smTileLab(tc,{kind:'ok',u:2*Math.PI+1e-9,v});n++;
   // across the seam: either the same tile continues (equal colour) or a neighbour starts (colour differs) — never a cut through a tile, which shows as θ=2π+ε ≠ θ=ε
   if(A.join()!==C.join())bad++;}}
 $('sm_target').value='view';return 'θ and θ + 2π differ on '+bad+' of '+n;}));
// 5. legends and renders for the boards (plane A, cylinder G, simplex S)
for(const [c,mode,extra,name] of [['A','colour',{},'leg_A_pos'],['A','tiling',{sm_tpat:'hex',sm_tcol:'index',sm_tn:'6'},'leg_A_hex'],['A','tiling',{sm_tpat:'square',sm_tcol:'layer',sm_tn:'6'},'leg_A_layer'],['G','tiling',{sm_tpat:'hex',sm_tcol:'index',sm_tn:'12'},'leg_G_hex'],['S','tiling',{sm_tpat:'tri',sm_tcol:'index',sm_tn:'6'},'leg_S_tri']]){
 await pn.evaluate(c=>select(CHARTS.find(x=>x.id===c)),c);await pn.waitForTimeout(300);await setv(pn,Object.assign({sm_mode:mode,sm_bright:'none',sm_target:'view'},extra));await pn.waitForTimeout(300);
 const w=await waitDone(pn);await pn.evaluate(()=>{playT=8});await pn.waitForTimeout(600);
 await (await pn.$('#sm_leg')).screenshot({path:name+'.png'});await (await pn.$('#cv')).screenshot({path:name.replace('leg','cv')+'_t8.png'});
 await pn.evaluate(()=>{playT=0});await pn.waitForTimeout(500);await (await pn.$('#cv')).screenshot({path:name.replace('leg','cv')+'_t0.png'});
 console.log(name,'wait',w,await pn.evaluate(()=>$('sm_status').textContent));}
await setv(pn,{sm_mode:'colour',sm_bright:'disp'});await pn.evaluate(()=>select(CHARTS.find(x=>x.id==='A')));await waitDone(pn);await pn.evaluate(()=>{playT=8});await pn.waitForTimeout(600);await (await pn.$('#cv')).screenshot({path:'cv_A_disp_t8.png'});
console.log('A disp graph:',await pn.evaluate(()=>$('sm_graph').innerText.replace(/\n/g,' / ')));
await setv(pn,{sm_mode:'tiling',sm_tpat:'square',sm_tcol:'index',sm_bright:'changed'});await pn.waitForTimeout(600);await (await pn.$('#cv')).screenshot({path:'cv_A_changed_t8.png'});
console.log('errs',errs.join('|')||'none');await b.close();})();
