import {defaultSettings,normalizeSettings,type Settings} from './types';
const KEY='daymark-appearance-v1';
const rgb=(hex:string)=>[1,3,5].map(i=>parseInt(hex.slice(i,i+2),16));
const hex=(rgb:number[])=>'#'+rgb.map(x=>Math.round(x).toString(16).padStart(2,'0')).join('');
const mix=(a:string,b:string,t:number)=>hex(rgb(a).map((v,i)=>v*(1-t)+rgb(b)[i]*t));
export function luminance(color:string){const [r,g,b]=rgb(color).map(c=>{const s=c/255;return s<=.04045?s/12.92:((s+.055)/1.055)**2.4;});return .2126*r+.7152*g+.0722*b;}
export function contrast(a:string,b:string){const x=luminance(a),y=luminance(b);return(Math.max(x,y)+.05)/(Math.min(x,y)+.05);}
export function palette(s:Settings,systemDark:boolean){
 const dark=s.themeMode==='system'?systemDark:s.themeMode==='dark';const bg=dark?s.backgroundDark:s.backgroundLight;
 const darkSurface=luminance(bg)<.35;
 const surface=mix(bg,'#ffffff',darkSurface?.04:.65);
 const score=(text:string)=>Math.min(contrast(bg,text),contrast(surface,text));
 const candidates=['#172033','#f3f5f9','#000000','#ffffff'];
 const fg=candidates.find(text=>score(text)>=4.5)??candidates.sort((a,b)=>score(b)-score(a))[0];
 const soft=mix(fg,bg,.22);const muted=score(soft)>=4.5?soft:fg;const border=mix(fg,bg,.84);
 const accentReadable=Math.min(contrast(bg,s.accentColor),contrast(surface,s.accentColor))>=4.5?s.accentColor:fg;
 const accentText=contrast(s.accentColor,'#ffffff')>=4.5?'#ffffff':'#000000';
 const selected=mix(surface,s.accentColor,.16);const selectedText=contrast(selected,fg)>=4.5?fg:contrast(selected,'#ffffff')>=4.5?'#ffffff':'#000000';
 return{dark,bg,fg,surface,muted,border,accent:s.accentColor,accentText,accentReadable,selected,selectedText,subtle:mix(bg,fg,.035),hover:mix(surface,fg,.055),danger:score('#be3535')>=4.5?'#be3535':score('#ffb4ab')>=4.5?'#ffb4ab':fg};
}
export function applyTheme(s:Settings,systemDark:boolean){const p=palette(s,systemDark);const root=document.documentElement;root.dataset.theme=p.dark?'dark':'light';root.style.colorScheme=luminance(p.bg)<.35?'dark':'light';for(const [name,value]of Object.entries(p)){if(typeof value==='string')root.style.setProperty('--'+name,value);}return p;}
export function applyCachedTheme(){try{applyTheme(normalizeSettings(JSON.parse(localStorage.getItem(KEY)||'{}')),matchMedia('(prefers-color-scheme: dark)').matches);}catch{applyTheme(defaultSettings,matchMedia('(prefers-color-scheme: dark)').matches);}}
export function cacheTheme(s:Settings){try{localStorage.setItem(KEY,JSON.stringify({themeMode:s.themeMode,accentColor:s.accentColor,backgroundLight:s.backgroundLight,backgroundDark:s.backgroundDark}));}catch{/* SQLite remains authoritative. */}}
