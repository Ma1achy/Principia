function lin(L,a,b){const l_=L+.3963377774*a+.2158037573*b,m_=L-.1055613458*a-.0638541728*b,s_=L-.0894841775*a-1.291485548*b;const l=l_**3,m=m_**3,s=s_**3;return [4.0767416621*l-3.3077115913*m+.2309699292*s,-1.2684380046*l+2.6097574011*m-.3413193965*s,-.0041960863*l-.7034186147*m+1.707614701*s];}
const ok=(L,a,b)=>lin(L,a,b).every(v=>v>=-1e-4&&v<=1+1e-4);
function worst(f,N=200){let bad=0,n=0;for(let i=0;i<=N;i++)for(let j=0;j<=N;j++){const [L,a,b]=f(i/N,j/N);n++;if(!ok(L,a,b))bad++;}return bad/n;}
// plane: hue arc x, lightness y
for(const C of [0.08,0.09,0.1,0.11]) for (const [L0,dL] of [[0.64,0.16],[0.64,0.14],[0.66,0.12]]) { const h0=0.6; const f=(u,v)=>{const h=h0+(u*2-1)*0.75*Math.PI;return [L0+dL*(v*2-1),C*Math.cos(h),C*Math.sin(h)]}; console.log('plane C',C,'L',L0,dL,worst(f)); }
for(const C of [0.08,0.09,0.095,0.1]) for(const dL of [0.16,0.2]){const f=(u,v)=>{const th=2*Math.PI*u,ph=Math.PI*v;const n=[Math.sin(ph)*Math.cos(th),Math.sin(ph)*Math.sin(th),Math.cos(ph)];return [0.64+dL*n[2],C*n[0],C*n[1]]};console.log('sphere C',C,'dL',dL,worst(f));}
for(const C of [0.09,0.1,0.11]){const f=(u,v)=>{const th=2*Math.PI*u;return [0.62+0.18*(2*v-1),C*Math.cos(th),C*Math.sin(th)]};console.log('cyl C',C,worst(f));}
for(const C of [0.07,0.08,0.09]) for (const [L0,dL] of [[0.64,0.14],[0.66,0.12],[0.66,0.1]]){const f=(u,v)=>{const th=2*Math.PI*u;return [L0+dL*(2*v-1),C*Math.cos(th),C*Math.sin(th)]};console.log('cyl2 C',C,L0,dL,worst(f));}
for(const C of [0.1,0.11,0.12]){const f=(u,v)=>{if(u+v>1)return [0.7,0,0];const m=[1-u-v,u,v];let a=0,b=0;[0,2*Math.PI/3,4*Math.PI/3].forEach((h,i)=>{a+=m[i]*Math.cos(h+.5);b+=m[i]*Math.sin(h+.5)});return [0.7,C*1.5*a,C*1.5*b]};console.log('simplex C',C,worst(f));}
