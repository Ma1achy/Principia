// OKLab -> linear sRGB (Ottosson), gamut check for a torus colour map
function lin(L,a,b){const l=(L+0.3963377774*a+0.2158037573*b)**3,m=(L-0.1055613458*a-0.0638541728*b)**3,s=(L-0.0894841775*a-1.2914855480*b)**3;
 return [4.0767416621*l-3.3077115913*m+0.2309699292*s,-1.2684380046*l+2.6097574011*m-0.3413193965*s,-0.0041960863*l-0.7034186147*m+1.7076147010*s];}
function ok(L0,rL,R,r,N=240){let bad=0;for(let i=0;i<N;i++)for(let j=0;j<N;j++){const th=2*Math.PI*i/N,ph=2*Math.PI*j/N,c=R+r*Math.cos(ph);const v=lin(L0+rL*Math.sin(ph),c*Math.cos(th),c*Math.sin(th));if(v.some(x=>x<-1e-9||x>1+1e-9))bad++;}return bad;}
// fix shape ratios, find the largest scale k that stays in gamut
for(const [L0,rL,R,r] of [[0.72,0.12,0.075,0.04],[0.72,0.10,0.08,0.035],[0.70,0.12,0.07,0.035],[0.72,0.13,0.07,0.035]]){
 let lo=0,hi=3;for(let it=0;it<30;it++){const k=(lo+hi)/2;ok(L0,rL,R*k,r*k,120)?hi=k:lo=k;}
 console.log(L0,rL,R,r,'max chroma scale',lo.toFixed(3),'-> R',(R*lo).toFixed(4),'r',(r*lo).toFixed(4),'bad at 0.98k',ok(L0,rL,R*lo*0.98,r*lo*0.98));}
