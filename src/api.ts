import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { defaultSettings, normalizeSettings, type Item, type Settings, type Alert, type SidebarState, type NotificationStatus, type NotificationTestState } from './types';
import { validateItem, instanceOnDay, parseDateKey } from './calendar';
export const native=isTauri();
const KEY='daymark-preview-v1';
interface Preview { items:Item[]; settings:Settings }
function preview():Preview { const raw=localStorage.getItem(KEY);const p:Preview=raw?JSON.parse(raw):{items:[],settings:defaultSettings};return {...p,settings:normalizeSettings(p.settings)}; }
function write(p:Preview){localStorage.setItem(KEY,JSON.stringify(p));window.dispatchEvent(new Event('preview-changed'));}
export const api={
 list:():Promise<Item[]>=>native?invoke('list_items'):Promise.resolve(preview().items),
 settings:():Promise<Settings>=>native?invoke('get_settings'):Promise.resolve(preview().settings),
 async save(item:Item):Promise<Item>{
  if(native)return invoke('save_item',{item});const error=validateItem(item);if(error)throw Error(error);
  const p=preview();let result={...item,title:item.title.trim(),version:item.version+1,snoozeUntil:null,occurrenceDate:null,scheduledAt:item.recurrence?null:item.scheduledAt,completedAt:item.recurrence?null:item.completedAt};
  if(!item.id){result={...result,id:crypto.randomUUID(),version:1,completedAt:null};p.items.push(result);}else{const idx=p.items.findIndex(i=>i.id===item.id);if(idx<0||p.items[idx].version!==item.version)throw Error('内容已在其他窗口更新，请关闭编辑框后重新编辑');result={...result,occurrenceStates:p.items[idx].occurrenceStates??{}};p.items[idx]=result;}
  write(p);return result;
 },
 async remove(item:Item){if(native)return invoke<void>('delete_item',{id:item.id,version:item.version});const p=preview();if(!p.items.some(i=>i.id===item.id&&i.version===item.version))throw Error('内容已更新，请刷新后重试');p.items=p.items.filter(i=>i.id!==item.id);write(p);},
 async complete(id:string,completed:boolean,occurrenceDate?:string|null){if(native)return invoke<void>('complete_item',{id,completed,occurrenceDate:occurrenceDate??null});const p=preview();const item=p.items.find(i=>i.id===id);if(!item)throw Error('任务已删除');const at=completed?Math.floor(Date.now()/1000):null;if(item.recurrence){const date=occurrenceDate?parseDateKey(occurrenceDate):null;if(!date||!instanceOnDay(item,date))throw Error('重复日期无效');item.occurrenceStates={...item.occurrenceStates,[occurrenceDate!]:{completedAt:at,snoozeUntil:null}};}else{item.completedAt=at;item.snoozeUntil=null;item.version++;}write(p);},
 async updateSettings(patch:Partial<Settings>):Promise<Settings>{if(native)return invoke('update_settings',{patch});const p=preview();const settings=normalizeSettings({...p.settings,...patch});write({...p,settings});return settings;},
 async subscribe(refresh:()=>void){if(native)return listen('data-changed',refresh);const storage=(e:StorageEvent)=>{if(e.key===KEY)refresh();};window.addEventListener('storage',storage);window.addEventListener('preview-changed',refresh);return()=>{window.removeEventListener('storage',storage);window.removeEventListener('preview-changed',refresh);};},
 sidebarState:()=>invoke<SidebarState>('sidebar_state'),
 sidebarAction:(action:'toggle'|'collapse'|'pin'|'activate')=>invoke<void>('sidebar_action',{action}),
 sidebarHover:(inside:boolean)=>invoke<void>('sidebar_hover',{inside}),
 sidebarResize:(height:number)=>invoke<void>('sidebar_resize',{height}),
 notificationStatus:():Promise<NotificationStatus>=>native?invoke('notification_status'):Promise.resolve({authorization:'unknown',alerts:'unknown',sound:'unknown',detail:'系统通知请在安装后的 Daymark 应用中测试。浏览器预览不发送系统通知。'}),
 requestNotification:()=>invoke<NotificationStatus>('request_notification_permission'),
 startNotificationTest:()=>invoke<NotificationTestState>('start_notification_test'),
 notificationTestState:()=>invoke<NotificationTestState>('notification_test_state'),
 cancelNotificationTest:()=>invoke<void>('cancel_notification_test'),
 openNotificationSettings:()=>invoke<void>('open_notification_settings'),
 alert:()=>invoke<Alert|null>('get_alert'),
 actAlert:(action:'complete'|'snooze'|'close')=>invoke<void>('act_alert',{action}),
 showMain:()=>invoke<void>('show_main'),
};
