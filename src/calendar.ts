import type { Item } from './types';
export const epoch = (d: Date) => Math.floor(d.getTime()/1000);
export function dayStart(d:Date) { return new Date(d.getFullYear(), d.getMonth(), d.getDate()); }
export function addDays(d:Date,n:number) { return new Date(d.getFullYear(),d.getMonth(),d.getDate()+n); }
export function sameDay(a:Date,b:Date) { return a.getFullYear()===b.getFullYear() && a.getMonth()===b.getMonth() && a.getDate()===b.getDate(); }
export function target(item:Item) { return item.kind==='task' ? item.scheduledAt??item.dueAt??item.startsAt : item.startsAt; }
export function onDay(item:Item, day:Date) {
 const start=epoch(dayStart(day)),end=epoch(addDays(day,1));
 if(item.kind==='task') { const at=item.scheduledAt??item.startsAt??item.dueAt; return at!==null && at>=start && at<end; }
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
 for(const n of [item.startsAt,item.endsAt,item.dueAt,item.scheduledAt??null])if(n!==null && (!Number.isFinite(n)||n<0||n>253402300799))return '时间无效';
 if(item.recurrence&&(!parseDateKey(item.recurrence.startDate)||!normalizeClock(item.recurrence.time)))return '请选择有效的每日提醒日期和时间';
 if(item.startsAt!==null && item.endsAt!==null && item.endsAt<=item.startsAt)return '结束时间必须晚于开始时间';
 return null;
}
export function eventLayout(items:Item[],day:Date) {
 const sorted=items.filter(i=>i.kind==='event'&&onDay(i,day)).sort((a,b)=>a.startsAt!-b.startsAt!);
 const groups:Item[][]=[];let end=0;
 for(const item of sorted){if(item.startsAt!>=end){groups.push([]);end=0;}groups.at(-1)!.push(item);end=Math.max(end,item.endsAt!);}
 return groups.flatMap(group=>{const lanes:number[]=[];const placed=group.map(item=>{let lane=lanes.findIndex(end=>end<=item.startsAt!);if(lane<0)lane=lanes.length;lanes[lane]=item.endsAt!;return{item,lane};});return placed.map(p=>({...p,lanes:lanes.length}));});
}

export function dateKey(date:Date){return `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,'0')}-${String(date.getDate()).padStart(2,'0')}`;}
export function parseDateKey(value:string){if(!/^\d{4}-\d{2}-\d{2}$/.test(value))return null;const [y,m,d]=value.split('-').map(Number);const date=new Date(y,m-1,d);return dateKey(date)===value?date:null;}
export function normalizeClock(value:string){const raw=value.trim().replace('：',':');let h:number,m=0;if(/^\d{1,2}$/.test(raw)){h=Number(raw);}else if(/^\d{3,4}$/.test(raw)){h=Number(raw.slice(0,-2));m=Number(raw.slice(-2));}else{const match=/^(\d{1,2}):(\d{1,2})$/.exec(raw);if(!match)return null;h=Number(match[1]);m=Number(match[2]);}return h<24&&m<60?`${String(h).padStart(2,'0')}:${String(m).padStart(2,'0')}`:null;}
export function localOccurrence(day:Date,time:string){const valid=normalizeClock(time);if(!valid)return null;const [h,m]=valid.split(':').map(Number);for(let n=0;n<=180;n++){const total=h*60+m+n;if(total>=1440)return null;const d=new Date(day.getFullYear(),day.getMonth(),day.getDate(),Math.floor(total/60),total%60);if(dateKey(d)===dateKey(day)&&d.getHours()===Math.floor(total/60)&&d.getMinutes()===total%60)return epoch(d);}return null;}
export function instanceOnDay(item:Item,day:Date):Item|null{if(!item.recurrence)return onDay(item,day)?item:null;const date=dateKey(day);if(date<item.recurrence.startDate)return null;const at=localOccurrence(day,item.recurrence.time);if(at===null)return null;const state=item.occurrenceStates?.[date];return {...item,scheduledAt:at,occurrenceDate:date,completedAt:state?.completedAt??null,snoozeUntil:state?.snoozeUntil??null};}
export const untimed=(item:Item)=>item.kind==='task'&&!item.recurrence&&target(item)===null;
export function instancesOnDay(items:Item[],day:Date,includeUntimed=false){return items.flatMap(item=>{const instance=instanceOnDay(item,day);return instance?[instance]:includeUntimed&&untimed(item)&&(item.completedAt===null||sameDay(new Date(item.completedAt*1000),day))?[item]:[];});}
export function nextItem(items:Item[],now:number){const day=new Date(now);const candidates=items.flatMap(item=>item.recurrence?[instanceOnDay(item,day),instanceOnDay(item,addDays(day,1))].filter((i):i is Item=>i!==null):[item]);return candidates.filter(i=>i.completedAt===null&&target(i)!>=now/1000).sort((a,b)=>target(a)!-target(b)!)[0];}
export const itemKey=(item:Item)=>`${item.id}@${item.occurrenceDate??''}`;
export function taskTime(item:Item){if(untimed(item))return '未定时';return `${clockTime(target(item))}${item.recurrence?' · 每天':item.dueAt?' 截止':''}`;}
