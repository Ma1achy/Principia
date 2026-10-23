const {chromium}=require('playwright');
(async()=>{const b=await chromium.launch();const errs=[];
const open=async f=>{const p=await b.newPage({viewport:{width:1400,height:1600}});p.on('pageerror',e=>errs.push(f+': '+e.message));await p.goto('file://'+process.cwd()+'/'+f);await p.waitForTimeout(600);return p;};
const p=await open('state_flow.html');
const setv=async o=>p.evaluate(o=>{for(const[k,v] of Object.entries(o)){$(k).value=v;$(k).dispatchEvent(new Event('change'));}SM.uiKey='';smSyncUI();},o);
const waitDone=async()=>{for(let i=0;i<1200;i++){const d=await p.evaluate(()=>SM.levels.length&&SM.levels[SM.levels.length-1].done);if(d)return i;await p.waitForTimeout(250);}return -1;};
const shot=async(n,t)=>{await p.evaluate(t=>{playT=t},t);await p.waitForTimeout(800);await (await p.$('#cv')).screenshot({path:n+'.png'});};
const graph=async()=>p.evaluate(()=>$('sm_graph').innerText.replace(/\n/g,' / '));
const sph=async (q,src)=>q.evaluate(src=>{if(src&&$('sm_src')){$('sm_src').value=src;}const tc=CHARTS.find(c=>c.id==='B');const out=[];let s=12345;const rnd=()=>{s=(s*16807)%2147483647;return s/2147483647};
 for(const c of [['colour'],['tiles','axes6','full','vmf','3'],['tiles','fib','golden','nearest','3'],['tiles','ico12','golden','topk','5'],['tiles','corner8','grad','vmf','8'],['bands']]){$('sm_mode').value=c[0];if(c[1]){$('sm_gen').innerHTML=SM_GENS.sphere.map(([k])=>`<option value="${k}">${k}</option>`).join('');$('sm_gen').value=c[1];$('sm_swatch').value=c[2];$('sm_kernel').value=c[3];$('sm_kappa').value=c[4];}
  $('sm_target').value='B';for(let k=0;k<400;k++){const th=rnd()*6.283,ph=Math.acos(2*rnd()-1);out.push(...smOccupantLab(tc,{kind:'ok',u:th,v:ph,n:[Math.sin(ph)*Math.cos(th),Math.sin(ph)*Math.sin(th),Math.cos(ph)]}));}}return out;},src);
{const po=await open('state_flow.v5.html');await po.evaluate(()=>select(CHARTS.find(c=>c.id==='B')));await p.evaluate(()=>select(CHARTS.find(c=>c.id==='B')));
 const A=await sph(p,'shape'),O=await sph(po,null);let ne=0;A.forEach((v,i)=>{if(v!==O[i])ne++});console.log('Shape n̂ node vs v5: values',A.length,'differing',ne);await po.close();
 await p.evaluate(()=>{$('sm_src').value='pos';$('sm_target').value='view';SM.uiKey='';smSyncUI();});}
// the same site blend under Chart position on the sphere is the same code path
console.log('Chart position site blend on sphere = Shape n̂ site blend:',await p.evaluate(()=>{const tc=CHARTS.find(c=>c.id==='B');$('sm_gen').value='axes6';$('sm_swatch').value='full';$('sm_kernel').value='vmf';$('sm_kappa').value='3';let worst=0;
 for(let k=0;k<300;k++){const th=Math.random()*6.283,ph=Math.acos(2*Math.random()-1),w={kind:'ok',u:th,v:ph,n:[Math.sin(ph)*Math.cos(th),Math.sin(ph)*Math.sin(th),Math.cos(ph)]};
  $('sm_src').value='shape';$('sm_mode').value='tiles';const a=smOccupantLab(tc,w);$('sm_src').value='pos';$('sm_mode').value='fsites';for(const x of [1,2,3])$('sm_l'+x).value='none';const b=smOccupantLab(tc,w);worst=Math.max(worst,...a.map((v,i)=>Math.abs(v-b[i])));}return worst;}));
await p.evaluate(()=>select(CHARTS.find(c=>c.id==='B')));await p.waitForTimeout(300);await setv({sm_src:'pos',sm_mode:'gradient'});console.log('B wait',await waitDone());
console.log('B options:',await p.evaluate(()=>[...$('sm_mode').options].filter(o=>!o.hidden).map(o=>o.text).join(', ')),'| layers:',await p.evaluate(()=>[...$('sm_l1').options].filter(o=>!o.hidden).map(o=>o.text).join(', ')));
for(const [n,o] of [['g3d',{sm_mode:'gradient',sm_grad:'3d'}],['glon',{sm_grad:'lon'}],['fs',{sm_mode:'fsites',sm_gen:'ico12',sm_swatch:'golden',sm_kernel:'vmf'}],['cells',{sm_mode:'palette',sm_tpat:'cells',sm_tn:'6'}],['latlon',{sm_tpat:'latlon'}],
  ['paper',{sm_mode:'gradient',sm_grad:'3d',sm_l1:'checker',sm_l1n:'6',sm_l1c:'white',sm_l1o:'0.25',sm_l2:'grid',sm_l2n:'6',sm_l2c:'dark',sm_l2o:'0.75'}],['celledge',{sm_l1:'none',sm_l2:'cells',sm_l2n:'6',sm_l2c:'dark',sm_l2o:'0.75'}]]){
 await setv(o);await shot('s_B_'+n+'_t0',0);await shot('s_B_'+n+'_t8',8);console.log(n,'|',await graph());}
await p.click('button[data-p=classic]');await p.waitForTimeout(500);console.log('classic |',await graph(),'| target row hidden',await p.evaluate(()=>$('sm_trow').hidden));
await p.evaluate(()=>{$('sm_src').value='pos';$('sm_src').dispatchEvent(new Event('change'));});await p.evaluate(()=>select(CHARTS.find(c=>c.id==='A')));await p.waitForTimeout(300);
for(const k of ['where','paper','basins','mixing']){await p.click(`button[data-p=${k}]`);await p.waitForTimeout(400);console.log('A preset',k,'|',await graph());}
await (await p.$('.side details:nth-of-type(2)')).screenshot({path:'s_card.png'});
console.log('errs',errs.join('|')||'none');await b.close();})();
