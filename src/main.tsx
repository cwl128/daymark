import React from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import { Sidebar,Reminder } from './NativeWindows';
import './styles.css';
const windowType=new URLSearchParams(location.search).get('window');
createRoot(document.getElementById('root')!).render(<React.StrictMode>{windowType==='sidebar'?<Sidebar/>:windowType==='reminder'?<Reminder/>:<App/>}</React.StrictMode>);
