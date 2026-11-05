import { api } from './api';
import { clockTime, countdown, onDay, sameDay, target } from './calendar';
import type { Item } from './types';
interface Props { items:Item[]; now:number; run:(fn:()=>Promise<unknown>)=>Promise<boolean>; edit?:(item:Item)=>void }
export function TodayPanel({items,now,run,edit}:Props){
 const today=new Date(now);const daily=items.filter(i=>onDay(i,today)||(i.completedAt!==null&&sameDay(new Date(i.completedAt*1000),today)));
 const tasks=daily.filter(i=>i.kind==='task');const done=daily.filter(i=>i.completedAt!==null).length;
 const upcoming=items.filter(i=>!i.completedAt&&target(i)!>=Math.floor(now/1000)).sort((a,b)=>target(a)!-target(b)!)[0];
 return <><section className="task-section"><h2>今日任务</h2>{tasks.length===0?<p className="muted">今天还没有任务。给自己留一点空间。</p>:tasks.map(item=><div className="task-row" key={item.id}><input type="checkbox" aria-label={`完成 ${item.title}`} checked={item.completedAt!==null} onChange={e=>void run(()=>api.complete(item.id,e.target.checked))}/><button className={`text-button ${item.completedAt?'done':''}`} disabled={!edit} onClick={()=>edit?.(item)}>{item.title}<small>{clockTime(item.dueAt??item.startsAt)}{item.dueAt?' 截止':''}</small></button></div>)}<div className="progress-label"><span>每日进度</span><span>{done} / {daily.length}</span></div><progress value={done} max={daily.length||1}/></section><section className="next-section"><h2>下一项</h2>{upcoming?<><p className="muted">{upcoming.title}</p><div className="countdown" aria-label="下一项倒计时">{countdown(target(upcoming)!-now/1000)}</div><small className="muted">{new Date(target(upcoming)!*1000).toLocaleString('zh-CN',{month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'})}{upcoming.kind==='task'?' 截止':' 开始'}</small></>:<p className="muted">没有即将开始的安排</p>}</section></>;
}
