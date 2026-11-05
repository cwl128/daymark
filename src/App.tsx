import { useState } from 'react';
import { api,native } from './api';
import { useSidebarState } from './NativeWindows';
import { addDays, epoch } from './calendar';
import { CalendarView,type View } from './CalendarView';
import { EntryDialog } from './EntryDialog';
import { useData,useNow } from './hooks';
import { SettingsView } from './SettingsView';
import { TodayPanel } from './TodayPanel';
import type { Item } from './types';
export function App(){
 const {items,settings,error,setError,loading,run}=useData();const now=useNow();
 const [date,setDate]=useState(()=>new Date()),[view,setView]=useState<View>('day'),[page,setPage]=useState<'calendar'|'settings'>('calendar'),[draft,setDraft]=useState<Item|null>(null);
 const sidebar=useSidebarState();
 function create(){const start=new Date(date);const current=new Date();start.setHours(current.getHours()+1,0,0,0);setDraft({id:'',title:'',notes:'',kind:'event',startsAt:epoch(start),endsAt:epoch(start)+1800,dueAt:null,completedAt:null,snoozeUntil:null,version:0});}
 function shift(direction:number){if(view==='month')setDate(new Date(date.getFullYear(),date.getMonth()+direction,1));else setDate(addDays(date,direction*(view==='week'?7:1)));}
 return <div className="app-shell"><nav className="navigation"><div className="brand"><span aria-hidden="true">▣</span> Daymark</div><button className={page==='calendar'?'selected':''} onClick={()=>setPage('calendar')}><span aria-hidden="true">▦</span>日历</button><button className={page==='settings'?'selected':''} onClick={()=>setPage('settings')}><span aria-hidden="true">⚙</span>设置</button><hr/><button disabled={!native} aria-pressed={sidebar} onClick={()=>void run(()=>api.sidebar(!sidebar))}><span aria-hidden="true">▯</span>悬浮侧边栏<span className={`switch ${sidebar?'on':''}`}/></button><div className="nav-bottom">免费 · 开源 · 离线</div></nav><div className="main-surface">{!native?<div className="preview-notice">浏览器预览 · 数据存于浏览器；桌面版使用 SQLite，并支持原生窗口和提醒</div>:null}{error?<div role="alert" className="error-banner">{error}<button aria-label="关闭错误" onClick={()=>setError('')}>×</button></div>:null}{page==='settings'?<SettingsView key={settings.notificationMinutes.join(',')} settings={settings} run={run}/>:<><header className="toolbar"><div className="date-title"><h1>{view==='month'?date.toLocaleDateString('zh-CN',{year:'numeric',month:'long'}):date.toLocaleDateString('zh-CN',{month:'long',day:'numeric',weekday:'long'})}</h1><div className="date-controls"><button aria-label="上一页" onClick={()=>shift(-1)}>‹</button><button aria-label="下一页" onClick={()=>shift(1)}>›</button><button onClick={()=>setDate(new Date())}>今天</button></div></div><div className="view-tabs" aria-label="日历视图">{(['day','week','month'] as const).map((v,i)=><button aria-pressed={view===v} className={view===v?'active':''} key={v} onClick={()=>setView(v)}>{['日','周','月'][i]}</button>)}</div><button className="primary create-button" onClick={create}>＋ 新建日程</button></header><div className="calendar-body">{loading?<p role="status">正在读取本地数据…</p>:<CalendarView items={items} date={date} view={view} edit={setDraft} now={now} select={d=>{setDate(d);setView('day');}}/>}<aside className="today-panel"><TodayPanel items={items} now={now} run={run} edit={setDraft}/></aside></div></>}<footer className="app-footer">所有数据仅保存在本机。</footer></div>{draft?<EntryDialog key={draft.id||'new'} item={draft} close={()=>setDraft(null)} run={run}/>:null}</div>;
}
