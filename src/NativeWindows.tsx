import { useEffect,useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { api,native } from './api';
import { clockTime,countdown,onDay,target } from './calendar';
import { useData,useNow } from './hooks';
import { TodayPanel } from './TodayPanel';
import type { Alert } from './types';
export function useSidebarState(){const [expanded,setExpanded]=useState(false);useEffect(()=>{if(!native)return;let disposed=false;let off:(()=>void)|undefined;void listen<boolean>('sidebar-state',e=>{if(!disposed)setExpanded(e.payload);}).then(async unlisten=>{if(disposed){unlisten();return;}off=unlisten;const value=await invoke<boolean>('sidebar_state');if(!disposed)setExpanded(value);}).catch(()=>{});return()=>{disposed=true;off?.();};},[]);return expanded;}
export function Sidebar(){
 const {items,settings,error,run}=useData();const now=useNow();const expanded=useSidebarState();
 return <div className="sidebar-surface" style={{flexDirection:settings.sidebarSide==='left'?'row-reverse':'row'}}><button className="edge-handle" aria-label={expanded?'收起侧边栏':'展开侧边栏'} aria-expanded={expanded} onClick={()=>void run(()=>api.sidebar(!expanded))}>{expanded?(settings.sidebarSide==='right'?'›':'‹'):(settings.sidebarSide==='right'?'‹':'›')}</button>{expanded?<div className="sidebar-content"><header><strong>今日安排</strong><button onClick={()=>void run(api.showMain)}>打开日历</button></header>{error?<p role="alert" className="error">{error}</p>:null}<TodayPanel items={items} now={now} run={run}/><section className="next-section"><h2>今日时间表</h2>{items.filter(i=>i.kind==='event'&&onDay(i,new Date(now))).map(i=><div className="task-row" key={i.id}><input type="checkbox" aria-label={`完成 ${i.title}`} checked={i.completedAt!==null} onChange={e=>void run(()=>api.complete(i.id,e.target.checked))}/><span className={i.completedAt?'done':''}>{i.title}<small className="muted" style={{display:'block'}}>{clockTime(i.startsAt)} – {clockTime(i.endsAt)}</small></span></div>)}</section></div>:null}</div>;
}
export function Reminder(){
 const [alert,setAlert]=useState<Alert|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false);const now=useNow();
 useEffect(()=>{let disposed=false;let off:(()=>void)|undefined;const refresh=()=>void api.alert().then(value=>{if(!disposed)setAlert(value);}).catch(e=>{if(!disposed)setError(String(e));});void listen('alert-changed',refresh).then(unlisten=>{if(disposed)unlisten();else{off=unlisten;refresh();}}).catch(e=>setError(String(e)));return()=>{disposed=true;off?.();};},[]);
 async function act(action:'complete'|'snooze'|'close'){setBusy(true);try{await api.actAlert(action);}catch(e){setError(String(e));}finally{setBusy(false);}}
 useEffect(()=>{const key=(e:KeyboardEvent)=>{if(e.key==='Escape'&&!busy)void act('close');};window.addEventListener('keydown',key);return()=>window.removeEventListener('keydown',key);},[busy]);
 return <main className="reminder-surface"><h1>日程提醒</h1>{alert?<><p className="muted">{alert.item.title} · {clockTime(target(alert.item))}</p><div className="reminder-timer">{target(alert.item)!*1000>now?countdown(target(alert.item)!-now/1000).replace(/^00:/,''):'已到时间'}</div><div className="reminder-actions"><button className="primary" disabled={busy} onClick={()=>void act('complete')}>完成</button><button disabled={busy} onClick={()=>void act('snooze')}>暂缓5分钟</button><button autoFocus disabled={busy} onClick={()=>void act('close')}>关闭</button></div><small>按 Escape 关闭 · 不影响系统快捷键</small></>:<p className="muted">暂无提醒</p>}{error?<p role="alert" className="error">{error}</p>:null}</main>;
}
