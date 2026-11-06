import {createContext,useContext,useEffect,useRef,useState,type ReactNode} from 'react';
import {api} from './api';
import {defaultSettings,type Item,type Settings} from './types';
type Data={items:Item[];settings:Settings;error:string;setError:(message:string)=>void;loading:boolean;run:(fn:()=>Promise<unknown>)=>Promise<boolean>;updateSettings:(patch:Partial<Settings>)=>Promise<boolean>};
const DataContext=createContext<Data|null>(null);
export function DataProvider({children}:{children:ReactNode}){
 const [items,setItems]=useState<Item[]>([]),[settings,setSettings]=useState<Settings>(defaultSettings),[error,setError]=useState(''),[loading,setLoading]=useState(true);
 const pending=useRef<{id:number;patch:Partial<Settings>}[]>([]),nextId=useRef(0),queue=useRef(Promise.resolve()),refreshRef=useRef(()=>{});
 const overlay=(stored:Settings)=>pending.current.reduce((s,p)=>({...s,...p.patch}),stored);
 useEffect(()=>{let disposed=false,sequence=0;let unlisten:(()=>void)|undefined;
  const refresh=async()=>{const seq=++sequence;try{const [rows,s]=await Promise.all([api.list(),api.settings()]);if(!disposed&&seq===sequence){setItems(rows);setSettings(overlay(s));setLoading(false);}}catch(e){if(!disposed){setError(String(e));setLoading(false);}}};
  refreshRef.current=()=>void refresh();void api.subscribe(()=>void refresh()).then(off=>{if(disposed)off();else{unlisten=off;void refresh();}}).catch(e=>{if(!disposed){setError(String(e));setLoading(false);}});
  return()=>{disposed=true;unlisten?.();};
 },[]);
 const run=async(fn:()=>Promise<unknown>)=>{try{await fn();return true;}catch(e){setError(String(e));return false;}};
 const updateSettings=(patch:Partial<Settings>):Promise<boolean>=>{
  const id=++nextId.current;pending.current.push({id,patch});setSettings(s=>({...s,...patch}));
  let success=true;
  const work=queue.current.then(async()=>{try{const s=await api.updateSettings(patch);pending.current=pending.current.filter(p=>p.id!==id);setSettings(overlay(s));}catch(e){success=false;pending.current=pending.current.filter(p=>p.id!==id);setError(String(e));refreshRef.current();}});
  queue.current=work;return work.then(()=>success);
 };
 return <DataContext.Provider value={{items,settings,error,setError,loading,run,updateSettings}}>{children}</DataContext.Provider>;
}
export function useData(){const data=useContext(DataContext);if(!data)throw Error('缺少数据上下文');return data;}
export function useNow(){const [now,setNow]=useState(Date.now());useEffect(()=>{const id=setInterval(()=>setNow(Date.now()),1000);return()=>clearInterval(id);},[]);return now;}
