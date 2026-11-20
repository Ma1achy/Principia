const c=require('./core.js'),{march}=require('./grid.js');
const rng=(()=>{let s=11;return()=>((s=(s*16807)%2147483647)/2147483647)})();
for (const T of [20]) for (const eps of [1e-9,1e-6,1e-3]) {
let big=0,n=0;const NK=Math.round(T/0.25)+1;
for(let i=0;i<300;i++){const th=6.28*rng(),ph=3.14*rng();const ic=c.decodeShape(th,ph);
 const run=(e)=>{const y=new Float64Array(12);for(let k=0;k<3;k++){y[2*k]=ic.r[k][0]*(1+(k==0?e:0));y[2*k+1]=ic.r[k][1];y[6+2*k]=ic.v[k][0];y[7+2*k]=ic.v[k][1];}const out=new Float64Array(NK*12);march(y,[1/3,1/3,1/3],T,0.25,1e-10,0.02,1e-3,out,0);return out.subarray((NK-1)*12,NK*12);};
 const a=run(0),b=run(eps);c.setM([1/3,1/3,1/3]);
 const ea=c.encode([[a[0],a[1]],[a[2],a[3]],[a[4],a[5]]],[[a[6],a[7]],[a[8],a[9]],[a[10],a[11]]]),eb=c.encode([[b[0],b[1]],[b[2],b[3]],[b[4],b[5]]],[[b[6],b[7]],[b[8],b[9]],[b[10],b[11]]]);
 const na=[Math.sin(ea.ph)*Math.cos(ea.th),Math.sin(ea.ph)*Math.sin(ea.th),Math.cos(ea.ph)],nb=[Math.sin(eb.ph)*Math.cos(eb.th),Math.sin(eb.ph)*Math.sin(eb.th),Math.cos(eb.ph)];
 const d=Math.hypot(na[0]-nb[0],na[1]-nb[1],na[2]-nb[2]);n++; if(d>0.02)big++;}
console.log('t',T,'perturbation',eps,'shape moved > 0.02 (about one 8-bit colour step x5):',big,'of',n);}
