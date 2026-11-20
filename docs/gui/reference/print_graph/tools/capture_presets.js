const {chromium}=require('playwright'); const fs=require('fs');
fs.mkdirSync('presets',{recursive:true});
(async()=>{const b=await chromium.launch({args:['--use-gl=swiftshader','--enable-unsafe-swiftshader']});
const p=await b.newPage({viewport:{width:1400,height:1000}}); await p.goto('http://localhost:'+(process.env.PORT||8766)+'/?'+Date.now());await p.waitForTimeout(6000);
await p.evaluate(()=>{const i=IMAGES.findIndex(x=>x[0]==='b'); document.querySelectorAll('#thumbs button')[i].click();}); await p.waitForTimeout(2500);
const names=await p.evaluate(()=>Object.keys(PRESETS)); const out=[];
const graphs={};
for(const n of names){ await p.evaluate(n=>selectPreset(n),n); await p.waitForTimeout(2600);
 const d=await p.evaluate(()=>{const c=[...document.querySelectorAll('canvas')].sort((a,b)=>b.width*b.height-a.width*a.height)[0];return c.toDataURL('image/png');});
 const f=n.replace(/[^a-z0-9]+/gi,'_').replace(/^_|_$/g,''); fs.writeFileSync('presets/'+f+'.png',Buffer.from(d.split(',')[1],'base64')); out.push([n,f]);
 graphs[n]=await p.evaluate(n=>graphFromPreset(n),n); }
fs.writeFileSync('presets/list.json',JSON.stringify(out)); fs.writeFileSync('presets_graphs.json',JSON.stringify(graphs,null,1)); console.log(out.length); await b.close();})();
