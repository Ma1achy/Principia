// one-to-one check: bin colours (OKLab, unclipped) into cells of size eps; any cell holding two points farther apart than D in the domain is a collision
function check(name, sample, colour, dist, eps=2e-3, D=0.2){const bins=new Map();let worst=0;
 for(const p of sample){const c=colour(p);const k=c.map(v=>Math.round(v/eps)).join(',');const b=bins.get(k);if(b){for(const q of b){const d=dist(p,q);if(d>worst)worst=d;}b.push(p);}else bins.set(k,[p]);}
 console.log(name.padEnd(28),'largest domain distance between same-colour points:',worst.toFixed(3), worst>D?'NOT one-to-one':'ok');}
const N=300, sph=[];for(let i=0;i<N;i++)for(let j=0;j<N;j++){const th=2*Math.PI*(i+.5)/N,ph=Math.acos(1-2*(j+.5)/N);sph.push([Math.sin(ph)*Math.cos(th),Math.sin(ph)*Math.sin(th),Math.cos(ph)]);}
const chord=(p,q)=>Math.hypot(p[0]-q[0],p[1]-q[1],p[2]-q[2]);
check('sphere map (linear in n)',sph,n=>[0.64+0.2*n[2],0.095*n[0],0.095*n[1]],chord);
// vMF six-pole, fixed L (dd_colouring 3.2), kappa 3, Full OKLAB hues
const poles=[[1,0,0],[-1,0,0],[0,1,0],[0,-1,0],[0,0,1],[0,0,-1]],hues=[0,180,120,300,240,60].map(h=>h*Math.PI/180);
for(const k of [1,3,6]) check('vMF fixed-L control, κ='+k,sph,n=>{let a=0,b=0,s=0;poles.forEach((p,i)=>{const w=Math.exp(k*(n[0]*p[0]+n[1]*p[1]+n[2]*p[2]));a+=w*Math.cos(hues[i]);b+=w*Math.sin(hues[i]);s+=w;});return [0.7,0.12*a/s,0.12*b/s];},chord);
const pl=[];for(let i=0;i<N;i++)for(let j=0;j<N;j++)pl.push([2*(i+.5)/N-1,2*(j+.5)/N-1]);
const d2=(p,q)=>Math.hypot(p[0]-q[0],p[1]-q[1]);
check('plane map (hue arc × L)',pl,([x,y])=>{const h=0.6+0.75*Math.PI*x;return [0.64+0.16*y,0.09*Math.cos(h),0.09*Math.sin(h)]},d2);
check('plain (a,b) square control',pl,([x,y])=>[0.7,0.09*x,0.09*y],d2);
check('closed hue circle control',pl,([x,y])=>{const h=Math.PI*x;return [0.64+0.16*y,0.09*Math.cos(h),0.09*Math.sin(h)]},d2);
const cy=[];for(let i=0;i<N;i++)for(let j=0;j<N;j++)cy.push([2*Math.PI*(i+.5)/N,2*(j+.5)/N-1]);
check('cylinder map',cy,([t,s])=>[0.66+0.12*s,0.09*Math.cos(t),0.09*Math.sin(t)],(p,q)=>Math.hypot(Math.min(Math.abs(p[0]-q[0]),2*Math.PI-Math.abs(p[0]-q[0])),p[1]-q[1]));
