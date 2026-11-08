import {clockTime,itemKey,taskTime} from './calendar';
import type {Item} from './types';
export function SidebarChecklist({items,complete}:{items:Item[];complete:(item:Item,checked:boolean)=>void}){
 return <ul className="sidebar-list" aria-label="待办勾选清单">{items.map(item=><li key={itemKey(item)}><label className={`sidebar-item ${item.completedAt!==null?'done':''}`}><input className="check-bullet" type="checkbox" aria-label={`完成 ${item.title}`} checked={item.completedAt!==null} onChange={e=>complete(item,e.target.checked)}/><span><strong>{item.title}</strong><small>{item.kind==='event'?`${clockTime(item.startsAt)} – ${clockTime(item.endsAt)}`:taskTime(item)}</small></span></label></li>)}</ul>;
}
