// standalone HTML of one frame: every <sc-for> expanded from its own list, filters/masks/blends intact
const fs=require('fs'); const [f,T,out,bg]=process.argv.slice(2);
const h=fs.readFileSync('p4/'+f+'.dc.html','utf8');
const js=h.match(/data-dc-script[^>]*>([\s\S]*?)<\/script>/)[1];
const C=new Function('DCLogic',js+'; return Component;')(class{constructor(){this.props={};}});
const c=new C(); c.state={t:+T}; const v=c.renderVals();
let svg=h.match(/<svg[\s\S]*?<\/svg>/)[0];
svg=svg.replace(/<sc-for list="\{\{(\w+)\}\}" as="(\w+)"[^>]*>([\s\S]*?)<\/sc-for>/g,(m,L,as,inner)=>v[L].map(x=>inner.replace(new RegExp('\\{\\{'+as+'\\.(\\w+)\\}\\}','g'),(mm,k)=>x[k])).join(''));
svg=svg.replace(/\{\{(\w+)\}\}/g,(m,k)=>v[k]);
fs.writeFileSync(out,`<!doctype html><html><body style="margin:0;background:${bg}">${svg}</body></html>`);
