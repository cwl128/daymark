import {useEffect,useRef,useState} from 'react';
import {listen} from '@tauri-apps/api/event';
import {api,native} from './api';
import {clockTime,countdown,instancesOnDay,nextItem,target} from './calendar';
import {SidebarChecklist} from './SidebarChecklist';
import {useData,useNow} from './hooks';
import type {Alert,SidebarState} from './types';
export function useSidebarState(){const [state,setState]=useState<SidebarState>({enabled:false,expanded:false,pinned:false});useEffect(()=>{if(!native)return;let disposed=false;let off:(()=>void)|undefined;void listen<SidebarState>('sidebar-state',e=>{if(!disposed)setState(e.payload);}).then(async unlisten=>{if(disposed){unlisten();return;}off=unlisten;const state=await api.sidebarState();if(!disposed)setState(state);}).catch(()=>{});return()=>{disposed=true;off?.();};},[]);return state;}
export function Sidebar(){
 const {items,error,run}=useData();const now=useNow();const state=useSidebarState();const content=useRef<HTMLDivElement>(null);
 const today=instancesOnDay(items,new Date(now),true);const done=today.filter(i=>i.completedAt!==null).length;
 const upcoming=nextItem(items,now);
 useEffect(()=>{if(!state.expanded||!content.current)return;const element=content.current;const resize=()=>void api.sidebarResize(element.scrollHeight+2).catch(()=>{});resize();const observer=new ResizeObserver(resize);observer.observe(element);return()=>observer.disconnect();},[state.expanded,items]);
 const hover=(inside:boolean)=>void run(()=>api.sidebarHover(inside));
 if(!state.expanded)return <button className="edge-handle" aria-label="展开侧栏" title="悬停或点击，展开今日安排" onPointerEnter={()=>hover(true)} onPointerLeave={()=>hover(false)} onClick={()=>void run(()=>api.sidebarAction('toggle'))}><span aria-hidden="true"/></button>;
 return <div className="sidebar-surface" onPointerEnter={()=>hover(true)} onPointerLeave={()=>hover(false)}><div className="sidebar-content" ref={content}><header><strong>今日安排</strong><div className="button-row"><button aria-label={state.pinned?'取消固定展开':'固定展开'} aria-pressed={state.pinned} className={state.pinned?'pin-active':''} onClick={()=>void run(()=>api.sidebarAction('pin'))}>⌖</button><button aria-label="收起侧栏" onClick={()=>void run(()=>api.sidebarAction('collapse'))}>×</button></div></header>
 {error?<p role="alert" className="error">{error}</p>:null}<section className="sidebar-next"><span className="muted">下一项</span>{upcoming?<><strong>{upcoming.title}</strong><span className="sidebar-countdown">{countdown(target(upcoming)!-now/1000)}</span><small className="muted">{clockTime(target(upcoming))}{upcoming.kind==='task'?(upcoming.dueAt?' 截止':' 提醒'):' 开始'}</small></>:<p className="muted">没有即将开始的安排</p>}</section><div className="sidebar-progress"><span>今日进度</span><span>{done} / {today.length}</span></div><progress value={done} max={today.length||1}/><section aria-label="今日清单">{today.length===0?<p className="muted">今天还没有安排。打开日历添加第一条。</p>:<SidebarChecklist items={today} complete={(item,checked)=>void run(()=>api.complete(item.id,checked,item.occurrenceDate))}/>}</section><button className="sidebar-open" onClick={()=>void run(api.showMain)}>打开日历 ↗</button></div></div>;
}
export function Reminder(){
 const [alert,setAlert]=useState<Alert|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false);const now=useNow();
 useEffect(()=>{let disposed=false;let off:(()=>void)|undefined;const refresh=()=>void api.alert().then(v=>{if(!disposed)setAlert(v);}).catch(e=>{if(!disposed)setError(String(e));});void listen('alert-changed',refresh).then(unlisten=>{if(disposed)unlisten();else{off=unlisten;refresh();}}).catch(e=>setError(String(e)));return()=>{disposed=true;off?.();};},[]);
 async function act(action:'complete'|'snooze'|'close'){setBusy(true);try{await api.actAlert(action);}catch(e){setError(String(e));}finally{setBusy(false);}}
 useEffect(()=>{const key=(e:KeyboardEvent)=>{if(e.key==='Escape'&&!busy)void act('close');};window.addEventListener('keydown',key);return()=>window.removeEventListener('keydown',key);},[busy]);
 return <main className="reminder-surface"><h1>日程提醒</h1>{alert?<><p className="muted">{alert.item.title} · {clockTime(target(alert.item))}</p><div className="reminder-timer">{target(alert.item)!*1000>now?countdown(target(alert.item)!-now/1000).replace(/^00:/,''):'已到时间'}</div><div className="reminder-actions"><button className="primary" disabled={busy} onClick={()=>void act('complete')}>完成</button><button disabled={busy} onClick={()=>void act('snooze')}>暂缓5分钟</button><button autoFocus disabled={busy} onClick={()=>void act('close')}>关闭</button></div><small>按 Escape 关闭 · 不影响系统快捷键</small></>:<p className="muted">暂无提醒</p>}{error?<p role="alert" className="error">{error}</p>:null}</main>;
}
