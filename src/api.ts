import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { defaultSettings, type Item, type Settings, type Alert } from './types';
import { validateItem } from './calendar';
export const native=isTauri();
const KEY='daymark-preview-v1';
interface Preview { items:Item[]; settings:Settings }
function preview():Preview { const raw=localStorage.getItem(KEY);return raw?JSON.parse(raw):{items:[],settings:defaultSettings}; }
function write(p:Preview){localStorage.setItem(KEY,JSON.stringify(p));window.dispatchEvent(new Event('preview-changed'));}
export const api={
 list:():Promise<Item[]>=>native?invoke('list_items'):Promise.resolve(preview().items),
 settings:():Promise<Settings>=>native?invoke('get_settings'):Promise.resolve(preview().settings),
 async save(item:Item):Promise<Item>{
  if(native)return invoke('save_item',{item});const error=validateItem(item);if(error)throw Error(error);
  const p=preview();let result={...item,title:item.title.trim(),version:item.version+1,snoozeUntil:null};
  if(!item.id){result={...result,id:crypto.randomUUID(),version:1,completedAt:null};p.items.push(result);}else{const idx=p.items.findIndex(i=>i.id===item.id);if(idx<0||p.items[idx].version!==item.version)throw Error('内容已在其他窗口更新，请关闭编辑框后重新编辑');p.items[idx]=result;}
  write(p);return result;
 },
 async remove(item:Item){if(native)return invoke<void>('delete_item',{id:item.id,version:item.version});const p=preview();if(!p.items.some(i=>i.id===item.id&&i.version===item.version))throw Error('内容已更新，请刷新后重试');p.items=p.items.filter(i=>i.id!==item.id);write(p);},
 async complete(id:string,completed:boolean){if(native)return invoke<void>('complete_item',{id,completed});const p=preview();p.items=p.items.map(i=>i.id===id?{...i,completedAt:completed?Math.floor(Date.now()/1000):null,snoozeUntil:null,version:i.version+1}:i);write(p);},
 async saveSettings(settings:Settings){if(native)return invoke<void>('save_settings',{settings});const p=preview();write({...p,settings});},
 async subscribe(refresh:()=>void){if(native)return listen('data-changed',refresh);const storage=(e:StorageEvent)=>{if(e.key===KEY)refresh();};window.addEventListener('storage',storage);window.addEventListener('preview-changed',refresh);return()=>{window.removeEventListener('storage',storage);window.removeEventListener('preview-changed',refresh);};},
 sidebar:(expanded:boolean)=>invoke<void>('set_sidebar',{expanded}),
 alert:()=>invoke<Alert|null>('get_alert'),
 actAlert:(action:'complete'|'snooze'|'close')=>invoke<void>('act_alert',{action}),
 showMain:()=>invoke<void>('show_main'),
};
