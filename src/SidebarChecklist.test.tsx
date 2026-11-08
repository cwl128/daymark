// @vitest-environment jsdom
import {afterEach,expect,it,vi} from 'vitest';
import {cleanup,fireEvent,render,screen,within} from '@testing-library/react';
import {SidebarChecklist} from './SidebarChecklist';
import {instancesOnDay} from './calendar';
import type {Item} from './types';
afterEach(cleanup);
it('renders today as a bullet checklist and changes only the selected task, including undo',()=>{
 const base:Item={id:'a',title:'买生活用品',notes:'',kind:'task',taskDate:'2026-11-07',startsAt:null,endsAt:null,dueAt:null,completedAt:null,snoozeUntil:null,version:1};
 const items=[base,{...base,id:'b',title:'整理书桌'},{...base,id:'tomorrow',taskDate:'2026-11-08',title:'明天再做'}];const complete=vi.fn();
 const today=instancesOnDay(items,new Date(2026,10,7),true);const view=render(<SidebarChecklist items={today} complete={complete}/>);
 const list=screen.getByRole('list',{name:'待办勾选清单'});expect(within(list).getAllByRole('listitem')).toHaveLength(2);expect(screen.queryByText('明天再做')).toBeNull();
 fireEvent.click(screen.getByRole('checkbox',{name:'完成 买生活用品'}));expect(complete).toHaveBeenCalledExactlyOnceWith(base,true);
 view.rerender(<SidebarChecklist items={[{...base,completedAt:123},today[1]]} complete={complete}/>);expect(screen.getByRole('checkbox',{name:'完成 买生活用品'})).toHaveProperty('checked',true);expect(screen.getByRole('checkbox',{name:'完成 整理书桌'})).toHaveProperty('checked',false);
 fireEvent.click(screen.getByRole('checkbox',{name:'完成 买生活用品'}));expect(complete).toHaveBeenLastCalledWith(expect.objectContaining({id:'a'}),false);
});
