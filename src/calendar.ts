import type { Item } from './types';
export const epoch = (d: Date) => Math.floor(d.getTime()/1000);
export function dayStart(d:Date) { return new Date(d.getFullYear(), d.getMonth(), d.getDate()); }
export function addDays(d:Date,n:number) { return new Date(d.getFullYear(),d.getMonth(),d.getDate()+n); }
export function sameDay(a:Date,b:Date) { return a.getFullYear()===b.getFullYear() && a.getMonth()===b.getMonth() && a.getDate()===b.getDate(); }
export function target(item:Item) { return item.kind==='task' ? item.dueAt??item.startsAt : item.startsAt; }
export function onDay(item:Item, day:Date) {
 const start=epoch(dayStart(day)),end=epoch(addDays(day,1));
 if(item.kind==='task') { const at=item.startsAt??item.dueAt; return at!==null && at>=start && at<end; }
 return item.startsAt!==null && item.endsAt!==null && item.startsAt<end && item.endsAt>start;
}
export function monthDays(d:Date) { const first=new Date(d.getFullYear(),d.getMonth(),1); const offset=(first.getDay()+6)%7; return Array.from({length:42},(_,i)=>addDays(first,i-offset)); }
export function weekDays(d:Date) { const first=addDays(d,-(d.getDay()+6)%7); return Array.from({length:7},(_,i)=>addDays(first,i)); }
export function inputTime(at:number|null) { if(at===null)return ''; const d=new Date(at*1000); const pad=(n:number)=>String(n).padStart(2,'0');return `${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`; }
export function parseTime(value:string) { return value ? epoch(new Date(value)) : null; }
export const clockTime=(at:number|null)=>at===null?'未安排':new Date(at*1000).toLocaleTimeString('zh-CN',{hour:'2-digit',minute:'2-digit',hour12:false});
export function countdown(seconds:number) { const t=Math.max(0,Math.floor(seconds)); const h=Math.floor(t/3600);return [h,Math.floor(t/60)%60,t%60].map(n=>String(n).padStart(2,'0')).join(':'); }
export function validateItem(item:Item):string|null {
 if(!item.title.trim()||item.title.trim().length>200)return '标题需为 1–200 个字符';
 if(item.notes.length>10000)return '备注不能超过 10000 字符';
 if(item.kind==='event' && (item.startsAt===null||item.endsAt===null))return '日程需要开始和结束时间';
 if(item.kind==='task' && target(item)===null)return '任务需要开始时间或截止时间';
 for(const n of [item.startsAt,item.endsAt,item.dueAt])if(n!==null && (!Number.isFinite(n)||n<0||n>253402300799))return '时间无效';
 if(item.startsAt!==null && item.endsAt!==null && item.endsAt<=item.startsAt)return '结束时间必须晚于开始时间';
 return null;
}
export function eventLayout(items:Item[],day:Date) {
 const sorted=items.filter(i=>i.kind==='event'&&onDay(i,day)).sort((a,b)=>a.startsAt!-b.startsAt!);
 const groups:Item[][]=[];let end=0;
 for(const item of sorted){if(item.startsAt!>=end){groups.push([]);end=0;}groups.at(-1)!.push(item);end=Math.max(end,item.endsAt!);}
 return groups.flatMap(group=>{const lanes:number[]=[];const placed=group.map(item=>{let lane=lanes.findIndex(end=>end<=item.startsAt!);if(lane<0)lane=lanes.length;lanes[lane]=item.endsAt!;return{item,lane};});return placed.map(p=>({...p,lanes:lanes.length}));});
}
