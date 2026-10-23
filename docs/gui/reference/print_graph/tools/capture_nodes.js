const {chromium}=require('playwright'); const fs=require('fs');
fs.mkdirSync(process.argv[2]||'shots',{recursive:true});
const IMG=process.env.IMG||'b', OUT=process.argv[2]||'shots';
(async()=>{const b=await chromium.launch({args:['--use-gl=swiftshader','--enable-unsafe-swiftshader','--ignore-gpu-blocklist']});
const p=await b.newPage({viewport:{width:1400,height:1000}});
const errs=[];p.on('pageerror',e=>errs.push(e.message));
await p.goto('http://localhost:'+(process.env.PORT||8766)+'/?'+Date.now());await p.waitForTimeout(6000);
await p.evaluate(k=>{const i=IMAGES.findIndex(x=>x[0]===k); document.querySelectorAll('#thumbs button')[i].click();},IMG); await p.waitForTimeout(2500);
const list=await p.evaluate(()=>{const out=[];const W={in:'img',render:'img',colour:'img',a:'img',flow:'fl',paper:'pap',density:'br',height:'br',mask:'br',field:'br',b:'br',ground:'gnd'};
 for(const [k,v] of Object.entries(NT)){ if(['source','out'].includes(v.cat)) continue; if(['image','output'].includes(k)) continue;
  const nodes={img:['image',30,40],fl:['flow',30,200,{}],pap:['paper',30,360,{}],br:['brightness',30,520,{}],gnd:['ground',30,680,{}],n:[k,300,80,{}],out:['output',560,100]};
  const edges=[['img','fl','in'],['img','br','in']];
  for(const inp of (v.inputs||[])) edges.push([W[inp]||'img','n',inp]);
  edges.push(['n','out','in']);
  PRESETS['__node_'+k]=P(nodes,edges); out.push([k,v.cat,v.label]); }
 return out;});
fs.writeFileSync(OUT+'/list.json',JSON.stringify(list));
for(const [k,cat,label] of list){ await p.evaluate(n=>selectPreset(n),'__node_'+k); await p.waitForTimeout(+(process.env.WAIT||2200));
 const d=await p.evaluate(()=>{const c=[...document.querySelectorAll('canvas')].sort((a,b)=>b.width*b.height-a.width*a.height)[0];return c.toDataURL('image/png');});
 fs.writeFileSync(`${OUT}/${cat}__${k}.png`,Buffer.from(d.split(',')[1],'base64'));
 const ge=await p.evaluate(()=>{const e=document.getElementById('glerr');return e?e.textContent:''}); if(ge) console.log('GLERR',k,ge.slice(0,300));
 process.stdout.write(k+' ');}
console.log('\nerrs',errs.slice(0,8).join('\n'));await b.close();})();
