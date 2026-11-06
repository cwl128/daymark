export interface Item { id: string; title: string; notes: string; kind: 'event'|'task'; startsAt: number|null; endsAt: number|null; dueAt: number|null; completedAt: number|null; snoozeUntil: number|null; version: number }
export interface Settings {
 notificationMinutes:number[];sound:boolean;fullscreen:boolean;sidebarSide:'left'|'right';
 sidebarEnabled:boolean;sidebarAutoHide:boolean;themeMode:'system'|'light'|'dark';
 accentColor:string;backgroundLight:string;backgroundDark:string;onboardingVersion:number;
}
export const defaultSettings:Settings={notificationMinutes:[10,5],sound:true,fullscreen:true,sidebarSide:'right',sidebarEnabled:false,sidebarAutoHide:true,themeMode:'system',accentColor:'#4f6bed',backgroundLight:'#f8fafc',backgroundDark:'#111827',onboardingVersion:0};
export const normalizeSettings=(settings:Partial<Settings>):Settings=>({...defaultSettings,...settings});
export interface Alert { item: Item; scheduledAt: number }
export interface SidebarState {enabled:boolean;expanded:boolean;pinned:boolean}
export interface NotificationStatus {authorization:'notDetermined'|'authorized'|'provisional'|'denied'|'unknown';alerts:'enabled'|'disabled'|'unknown';sound:'enabled'|'disabled'|'unknown';detail:string}
export interface NotificationTestState{id:string;phase:'idle'|'countdown'|'accepted'|'failed';dueAt:number|null;detail:string}
