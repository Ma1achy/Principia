const c=require('./core.js'),{march}=require('./grid.js');
const rng=(()=>{let s=5;return()=>((s=(s*16807)%2147483647)/2147483647)})();
const AMIN=0.05, logit=x=>Math.log(x/(1-x)), th=x=>Math.tanh(x/3);
function where(y){c.setM([1/3,1/3,1/3]);const e=c.encode([[y[0],y[1]],[y[2],y[3]],[y[4],y[5]]],[[y[6],y[7]],[y[8],y[9]],[y[10],y[11]]]);return e;}
function clampOf(e){const al=0.5*Math.acos(Math.max(-1,Math.min(1,Math.sin(e.ph)*Math.cos(e.th))));return [Number.isFinite(e.za)?th(e.za):(al<Math.PI/4?-1:1), Number.isFinite(e.zb)?th(e.zb):(e.zb>0?1:-1)];}
const T=8; const res={};
for (const DT of [0.25,0.005]) { const NK=Math.round(T/DT)+1; let n=0, du=[], dv=[]; const rr=(()=>{let s=5;return()=>((s=(s*16807)%2147483647)/2147483647)})();
 for(let i=0;i<400;i++){const za=-3+6*rr(),zb=-3+6*rr();const ic=c.decodeLatent(za,zb,[0,0,0,0]);const y=new Float64Array(12);for(let k=0;k<3;k++){y[2*k]=ic.r[k][0];y[2*k+1]=ic.r[k][1];y[6+2*k]=ic.v[k][0];y[7+2*k]=ic.v[k][1];}
  const out=new Float64Array(NK*12);march(y,[1/3,1/3,1/3],T,DT,1e-10,0.02,1e-3,out,0);
  const at=k=>out.subarray(k*12,k*12+12); const eN=where(at(NK-1)); if(Number.isFinite(eN.za)&&Number.isFinite(eN.zb)) continue;
  let last=null; for(let k=NK-2;k>=0;k--){const e=where(at(k)); if(Number.isFinite(e.za)&&Number.isFinite(e.zb)){last=[th(e.za),th(e.zb)];break;}} if(!last) continue;
  const cl=clampOf(eN); n++; du.push(Math.abs(cl[0]-last[0])); dv.push(Math.abs(cl[1]-last[1])); }
 const q=(a,f)=>{a=a.slice().sort((x,y)=>x-y);return +a[Math.floor(f*(a.length-1))].toFixed(3)};
 console.log('DT',DT,'left at t=8:',n,' |Δs_α| (the axis it left by) p50',q(du,.5),'p90',q(du,.9),'  |Δs_β| (the other axis) p50',q(dv,.5),'p90',q(dv,.9),'max',q(dv,1)); }
