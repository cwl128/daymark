import { useEffect, useState } from 'react';
import { api } from './api';
import { defaultSettings, type Item, type Settings } from './types';
export function useData(){
 const [items,setItems]=useState<Item[]>([]),[settings,setSettings]=useState<Settings>(defaultSettings),[error,setError]=useState(''),[loading,setLoading]=useState(true);
 useEffect(()=>{let disposed=false;let sequence=0;let unlisten:(()=>void)|undefined;
 const refresh=async()=>{const current=++sequence;try{const [rows,s]=await Promise.all([api.list(),api.settings()]);if(!disposed&&current===sequence){setItems(rows);setSettings(s);setLoading(false);}}catch(e){if(!disposed){setError(String(e));setLoading(false);}}};
 void api.subscribe(()=>void refresh()).then(off=>{if(disposed)off();else{unlisten=off;void refresh();}}).catch(e=>{if(!disposed){setError(String(e));setLoading(false);}});
 return()=>{disposed=true;unlisten?.();};},[]);
 const run=async(fn:()=>Promise<unknown>)=>{try{await fn();return true;}catch(e){setError(String(e));return false;}};
 return{items,settings,error,setError,loading,run};
}
export function useNow(){const [now,setNow]=useState(Date.now());useEffect(()=>{const id=setInterval(()=>setNow(Date.now()),1000);return()=>clearInterval(id);},[]);return now;}
