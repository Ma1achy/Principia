import json, subprocess
WF=json.load(open('word_frames.json'))
H={'WordFlatSub':360,'WordFlatSubNight':360,'WordMixedSub':420,'Word3DSub':480}
for k in H:
    WF.pop(k,None)
json.dump(WF,open('word_frames.json','w'))
subprocess.run(['python3','wordmarks.py'],check=True,capture_output=True)
for k,h in H.items():
    m=subprocess.check_output(['node','-e',f'''
const fs=require('fs');const hh=fs.readFileSync('p4/{k}.dc.html','utf8');
const js=hh.match(/data-dc-script[^>]*>([\\s\\S]*?)<\\/script>/)[1];
const C=new Function('DCLogic',js+'; return Component;')(class{{constructor(){{this.props={{}};}}}});
let x0=1e9,x1=-1e9,y0=1e9,y1=-1e9;
for (let t=0;t<36;t+=0.75){{ const c=new C(); c.state={{t}}; const v=c.renderVals();
  for (const it of [...v.itemsBack,...v.itemsMid,...v.itemsB]) {{ if (it.d.includes('a')) {{ const m=it.d.match(/^M(-?[\\d.]+),(-?[\\d.]+)a(-?[\\d.]+)/); if(!m) continue; const r=+m[3]*0.55, cx=+m[1]+(+m[3]), cy=+m[2]; x0=Math.min(x0,cx-r); x1=Math.max(x1,cx+r); y0=Math.min(y0,cy-r); y1=Math.max(y1,cy+r); continue; }}
    for (const m of it.d.matchAll(/(-?[\\d.]+),(-?[\\d.]+)/g)) {{ x0=Math.min(x0,+m[1]); x1=Math.max(x1,+m[1]); y0=Math.min(y0,+m[2]); y1=Math.max(y1,+m[2]); }} }} }}
console.log(JSON.stringify([x0,x1,y0,y1]));'''])
    b=json.loads(m); K,CX,CY=60,200,h//2
    w=b[1]-b[0]; hh=b[3]-b[2]; s=min(1500/w,(h-60)/hh); bx=(b[0]+b[1])/2-CX; by=(b[2]+b[3])/2-CY
    WF[k]=[round(K*s),round(800-s*bx),round(h/2-s*by)]
    print(k,[round(x) for x in b],WF[k])
json.dump(WF,open('word_frames.json','w'))
subprocess.run(['python3','wordmarks.py'],check=True)
