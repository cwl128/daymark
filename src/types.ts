export interface Item { id: string; title: string; notes: string; kind: 'event'|'task'; startsAt: number|null; endsAt: number|null; dueAt: number|null; completedAt: number|null; snoozeUntil: number|null; version: number }
export interface Settings { notificationMinutes: number[]; sound: boolean; fullscreen: boolean; sidebarSide: 'left'|'right' }
export const defaultSettings: Settings = { notificationMinutes: [10,5], sound: true, fullscreen: true, sidebarSide: 'right' };
export interface Alert { item: Item; scheduledAt: number }
