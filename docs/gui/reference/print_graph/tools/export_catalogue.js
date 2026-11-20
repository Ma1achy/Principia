const {chromium}=require('playwright'); const fs=require('fs');
(async()=>{const b=await chromium.launch({args:['--use-gl=swiftshader','--enable-unsafe-swiftshader']});
const p=await b.newPage(); await p.goto('http://localhost:'+(process.env.PORT||8766)+'/?'+Date.now());await p.waitForTimeout(5000);
const c=await p.evaluate(()=>{const o={};for(const [k,v] of Object.entries(NT)){o[k]={category:v.cat,category_label:CATS[v.cat]||v.cat,label:v.label,inputs:v.inputs||[],doc:v.doc||'',params:v.params||{},passes:(v.passes||[]).length};}return {categories:CATS,nodes:o};});
fs.writeFileSync('node_catalogue.json',JSON.stringify(c,null,1)); console.log(Object.keys(c.nodes).length); await b.close();})();
