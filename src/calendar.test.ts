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

import {dateKey,instanceOnDay,instancesOnDay,localOccurrence,nextItem,normalizeClock,target} from './calendar';
describe('simple tasks and daily routines',()=>{
 const plain:Item={id:'call',title:'和爸妈打电话',notes:'',kind:'task',startsAt:null,endsAt:null,dueAt:null,completedAt:null,snoozeUntil:null,version:1};
 it('accepts title-only and a single scheduled time without a fake deadline',()=>{expect(validateItem(plain)).toBeNull();const at=epoch(new Date(2026,10,7,21));expect(validateItem({...plain,scheduledAt:at})).toBeNull();expect(target({...plain,scheduledAt:at})).toBe(at);expect(instancesOnDay([plain],new Date(2026,10,7),true)).toHaveLength(1);expect(instancesOnDay([plain],new Date(2026,10,7))).toHaveLength(0);});
 it('normalizes convenient 24-hour entry and rejects impossible clocks',()=>{for(const time of ['21','2100','21:00','21：00'])expect(normalizeClock(time)).toBe('21:00');expect(normalizeClock('9:3')).toBe('09:03');expect(normalizeClock('24:00')).toBeNull();expect(normalizeClock('21:99')).toBeNull();});
 it('finishing tonight does not finish tomorrow or move its time',()=>{const day=new Date(2026,10,7);const item={...plain,recurrence:{startDate:dateKey(day),time:'21:00'},occurrenceStates:{'2026-11-07':{completedAt:123,snoozeUntil:null}}};const today=instanceOnDay(item,day)!;const tomorrow=instanceOnDay(item,addDays(day,1))!;expect(today.completedAt).toBe(123);expect(tomorrow.completedAt).toBeNull();expect(new Date(target(tomorrow)!*1000).getHours()).toBe(21);expect(instanceOnDay(item,addDays(day,-1))).toBeNull();expect(nextItem([item],new Date(2026,10,7,20).getTime())?.occurrenceDate).toBe('2026-11-08');});
 it('uses local calendar dates across month/year changes and excludes old completed untimed tasks',()=>{const day=new Date(2026,11,31);const item={...plain,recurrence:{startDate:'2026-12-31',time:'21:00'}};const tomorrow=instanceOnDay(item,addDays(day,1))!;expect(tomorrow.occurrenceDate).toBe('2027-01-01');expect(instancesOnDay([{...plain,completedAt:epoch(day)}],addDays(day,1),true)).toHaveLength(0);expect(localOccurrence(day,'21:00')).toBe(epoch(new Date(2026,11,31,21)));});
});
