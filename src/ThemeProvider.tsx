import {useEffect,useLayoutEffect,useRef,useState,type ReactNode} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {native} from './api';
import {useData} from './hooks';
import {applyTheme,cacheTheme} from './theme';
export function ThemeProvider({children}:{children:ReactNode}){
 const {settings,loading,setError}=useData();const [systemDark,setSystemDark]=useState(()=>matchMedia('(prefers-color-scheme: dark)').matches);const ready=useRef(false);
 useEffect(()=>{const query=matchMedia('(prefers-color-scheme: dark)');const update=()=>setSystemDark(query.matches);query.addEventListener('change',update);return()=>query.removeEventListener('change',update);},[]);
 useLayoutEffect(()=>{if(loading)return;const p=applyTheme(settings,systemDark);cacheTheme(settings);
  if(native){void invoke('sync_theme',{mode:settings.themeMode,background:p.bg}).catch(e=>setError(String(e))).finally(()=>{if(!ready.current){ready.current=true;void invoke('ui_ready').catch(e=>setError(String(e)));}});}
 },[settings.themeMode,settings.accentColor,settings.backgroundLight,settings.backgroundDark,loading,systemDark]);
 return children;
}
