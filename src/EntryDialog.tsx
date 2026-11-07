import {useEffect,useRef,useState} from 'react';
import {api} from './api';
import {dateKey,epoch,inputTime,normalizeClock,parseDateKey,parseTime,target,validateItem} from './calendar';
import type {Item} from './types';
interface Props {item:Item;close:()=>void;run:(fn:()=>Promise<unknown>)=>Promise<boolean>;saved?:(item:Item)=>void}
export function EntryDialog({item,close,run,saved}:Props){
 const dialog=useRef<HTMLDialogElement>(null);
 const [draft,setDraft]=useState(item),[error,setError]=useState(''),[busy,setBusy]=useState(false),[deleting,setDeleting]=useState(false);
 const initialAt=target(item);const initialDate=initialAt===null?new Date():new Date(initialAt*1000);
 const initialClock=item.recurrence?.time??(item.id&&initialAt!==null?`${String(initialDate.getHours()).padStart(2,'0')}:${String(initialDate.getMinutes()).padStart(2,'0')}`:'');
 const [clock,setClock]=useState(initialClock),[date,setDate]=useState(item.recurrence?.startDate??dateKey(initialDate));
 const [repeat,setRepeat]=useState<'none'|'daily'>(item.recurrence?'daily':'none'),[deadline,setDeadline]=useState(item.dueAt!==null);
 useEffect(()=>{dialog.current?.showModal();},[]);
 async function save(){
  let result={...draft};
  if(draft.kind==='task'){
   const time=normalizeClock(clock);const day=parseDateKey(date);
   if(clock.trim()&&!time){setError('请输入有效时间，例如 21:00，也可输入 2100 或 21');return;}
   if(repeat==='daily'&&!time){setError('请选择每天提醒的时间，例如 21:00');return;}
   if(time&&!day){setError('请选择有效日期');return;}
   const unchanged=Boolean(item.id)&&item.kind==='task'&&!item.recurrence&&repeat==='none'&&time===normalizeClock(initialClock)&&date===dateKey(initialDate)&&deadline===(item.dueAt!==null);
   const at=time&&day?epoch(new Date(`${date}T${time}:00`)):null;
   result={...draft,startsAt:unchanged?item.startsAt:null,endsAt:unchanged?item.endsAt:null,dueAt:repeat==='none'&&deadline?(unchanged?item.dueAt:at):null,scheduledAt:repeat==='none'&&!deadline?(unchanged&&item.startsAt!==null?item.scheduledAt??null:unchanged&&item.scheduledAt!=null?item.scheduledAt:at):null,recurrence:repeat==='daily'?{startDate:date,time:time!}:null};
  }else result={...draft,scheduledAt:null,recurrence:null};
  const message=validateItem(result);if(message){setError(message);return;}
  setBusy(true);if(await run(async()=>{const stored=await api.save(result);saved?.(stored);}))close();else setBusy(false);
 }
 async function remove(){setBusy(true);if(await run(()=>api.remove(item)))close();else setBusy(false);}
 function changeKind(kind:Item['kind']){setDraft(d=>{const start=target(d)??Math.floor(Date.now()/1000)+3600;return {...d,kind,startsAt:kind==='event'?(d.startsAt??start):d.startsAt,endsAt:kind==='event'?(d.endsAt??start+1800):d.endsAt};});}
 const daily=draft.kind==='task'&&repeat==='daily';
 return <dialog ref={dialog} onCancel={e=>{e.preventDefault();if(!busy)close();}} aria-labelledby="entry-title"><form onSubmit={e=>{e.preventDefault();void save();}}>
 <header><h2 id="entry-title">{item.id?'编辑安排':draft.kind==='task'?'添加任务':'添加日程'}</h2><button type="button" aria-label="关闭编辑" onClick={close} disabled={busy}>×</button></header>
 <label>标题<input autoFocus required placeholder={draft.kind==='task'?'例如：和爸妈打电话':'例如：下午的课程'} maxLength={200} value={draft.title} onChange={e=>setDraft({...draft,title:e.target.value})}/></label>
 <label>类型<select aria-label="类型" value={draft.kind} onChange={e=>changeKind(e.target.value as Item['kind'])}><option value="event">日程 · 有起止时间</option><option value="task">任务 · 可只填标题或一个时间</option></select></label>
 {draft.kind==='event'?<>{(['startsAt','endsAt'] as const).map((field,i)=><label key={field}>{['开始时间','结束时间'][i]}<input required type="datetime-local" value={inputTime(draft[field])} onChange={e=>setDraft({...draft,[field]:parseTime(e.target.value)})}/></label>)}</>:<>
 <label>提醒时间<input aria-label="提醒时间" inputMode="numeric" autoComplete="off" placeholder="选填，例如 21:00" value={clock} onChange={e=>setClock(e.target.value)} onBlur={()=>{const value=normalizeClock(clock);if(value)setClock(value);}}/></label>
 <p className="entry-hint muted">24 小时制。不填时间，就是普通待办。</p>
 <label>重复<select aria-label="重复" value={repeat} onChange={e=>setRepeat(e.target.value as 'none'|'daily')}><option value="none">仅一次</option><option value="daily">每天</option></select></label>
 {clock.trim()&&repeat==='none'?<label>日期<input aria-label="提醒日期" type="date" value={date} onChange={e=>setDate(e.target.value)}/></label>:null}
 {daily?<p className="repeat-summary">{normalizeClock(clock)?`每天 ${normalizeClock(clock)} 提醒`:'设置一个每天提醒的时间'}。今天打勾完成后，明天仍会出现。</p>:null}
 </>}
 <details className="entry-options"><summary>更多选项</summary>{daily?<label>开始日期<input aria-label="重复开始日期" type="date" value={date} onChange={e=>setDate(e.target.value)}/></label>:null}{draft.kind==='task'&&!daily?<label className="entry-check"><input type="checkbox" checked={deadline} onChange={e=>setDeadline(e.target.checked)}/>将这个时间设为截止时间</label>:null}{draft.kind==='event'?<label>截止时间<input type="datetime-local" value={inputTime(draft.dueAt)} onChange={e=>setDraft({...draft,dueAt:parseTime(e.target.value)})}/></label>:null}<label>备注<textarea rows={3} maxLength={10000} value={draft.notes} onChange={e=>setDraft({...draft,notes:e.target.value})}/></label><p className="muted">按系统时区显示：{Intl.DateTimeFormat().resolvedOptions().timeZone}</p></details>
 {item.id&&item.recurrence?<p className="entry-hint muted">修改会应用到整个每日待办；勾选完成只影响当前这一天。</p>:null}
 {error?<p role="alert" className="error">{error}</p>:null}
 {deleting?<div className="delete-confirm"><p>{item.recurrence?'删除整个每日待办及其完成记录？':`确认删除「${item.title}」？`}</p><button type="button" disabled={busy} className="danger" onClick={()=>void remove()}>确认删除</button><button type="button" onClick={()=>setDeleting(false)}>保留</button></div>:null}
 <footer>{item.id?<button type="button" className="danger" disabled={busy} onClick={()=>setDeleting(true)}>{item.recurrence?'删除每日待办':'删除'}</button>:null}{item.id?<button type="button" disabled={busy} onClick={async()=>{setBusy(true);if(await run(()=>api.complete(item.id,item.completedAt===null,item.occurrenceDate)))close();else setBusy(false);}}>{item.completedAt?'标为未完成':item.recurrence?'完成这一天':'标为完成'}</button>:null}<div className="spacer"/><button type="button" disabled={busy} onClick={close}>取消</button><button className="primary" disabled={busy}>{busy?'保存中…':item.id?'保存':'创建'}</button></footer>
 </form></dialog>;
}
