import {describe,it,expect} from 'vitest';
import {addDays,dayStart,epoch,eventLayout,monthDays,onDay,validateItem,weekDays} from './calendar';
import type {Item} from './types';
const event=(start:Date,end:Date):Item=>({id:'a',title:'测试',notes:'',kind:'event',startsAt:epoch(start),endsAt:epoch(end),dueAt:null,completedAt:null,snoozeUntil:null,version:1});
describe('calendar boundaries',()=>{
 it('includes overnight events on both days but excludes exact ending midnight',()=>{const a=new Date(2026,10,5,23),b=new Date(2026,10,6,1);const item=event(a,b);expect(onDay(item,a)).toBe(true);expect(onDay(item,b)).toBe(true);item.endsAt=epoch(dayStart(b));expect(onDay(item,b)).toBe(false);});
 it('adds calendar days across DST without assuming 24 hours',()=>{const before=new Date(2026,10,1);const next=addDays(before,1);expect(next.getDate()).toBe(2);expect(next.getHours()).toBe(0);});
 it('places deadline-only tasks on the deadline day',()=>{const d=new Date(2026,10,5,18);const item={...event(d,addDays(d,1)),kind:'task' as const,startsAt:null,endsAt:null,dueAt:epoch(d)};expect(onDay(item,d)).toBe(true);expect(onDay(item,addDays(d,1))).toBe(false);expect(validateItem(item)).toBeNull();});
 it('builds a Monday-first week at a month boundary',()=>{const days=weekDays(new Date(2026,10,1));expect(days[0].getDay()).toBe(1);expect(days.at(-1)!.getDate()).toBe(1);});
 it('builds a complete 42-cell month grid',()=>{const days=monthDays(new Date(2026,10,5));expect(days).toHaveLength(42);expect(days[0].getDay()).toBe(1);expect(days.filter(d=>d.getMonth()===10)).toHaveLength(30);});
 it('uses separate lanes for overlapping events and reuses nonoverlapping lanes',()=>{const at=(h:number)=>new Date(2026,10,5,h);const a=event(at(9),at(11)),b={...event(at(10),at(12)),id:'b'},c={...event(at(11),at(12)),id:'c'};const layout=eventLayout([a,b,c],at(9));expect(layout.map(l=>l.lane)).toEqual([0,1,0]);expect(layout.every(l=>l.lanes===2)).toBe(true);});
 it('rejects empty titles, missing event times and reversed ranges',()=>{const a=new Date(2026,10,5,9),b=new Date(2026,10,5,8);expect(validateItem(event(a,b))).toContain('结束');expect(validateItem({...event(b,a),title:' '})).toContain('标题');expect(validateItem({...event(b,a),startsAt:null})).toContain('开始');});
});
