import {useEffect,useRef,useState} from 'react';
import {useData} from './hooks';
import {native} from './api';
import {NotificationTest} from './NotificationTest';
export function Onboarding({close}:{close:()=>void}){
 const {settings,updateSettings}=useData();const [step,setStep]=useState(0),[busy,setBusy]=useState(false),[error,setError]=useState('');const dialog=useRef<HTMLDialogElement>(null);
 useEffect(()=>{if(!dialog.current?.open)dialog.current?.showModal();},[]);
 async function finish(){setBusy(true);if(await updateSettings({onboardingVersion:1}))close();else{setError('暂时无法保存引导进度，请重试。');setBusy(false);}}
 return <dialog ref={dialog} className="onboarding" aria-labelledby="guide-title" onCancel={e=>{e.preventDefault();if(!busy)void finish();}}><header><h2 id="guide-title">使用引导</h2><button aria-label="跳过引导" disabled={busy} onClick={()=>void finish()}>×</button></header><ol className="guide-steps">{['添加安排','侧栏收放','通知自测'].map((s,i)=><li key={s} aria-current={i===step?'step':undefined}><span>{i+1}</span>{s}</li>)}</ol>
 <div className="guide-content">{step===0?<><h3>把今天的安排放在一起</h3><p>选择“添加日程”来安排有开始、结束时间的活动；选择“添加任务”来记录待办。只填标题即可；需要提醒时填一个时间，也可以选择每天重复。</p><div className="guide-example"><strong>日程：下午的课程</strong><span>14:00 – 15:00，有明确的时间段</span><strong>任务：和爸妈打电话</strong><span>每天 21:00，一个提醒时间；每天分别打勾</span></div><p className="muted">这只是说明，不会往你的日历里添加内容。所有安排只保存在本机。</p></>:step===1?<><h3>需要时出现，离开后收起</h3><div className="guide-sidebar-demo"><span className="mini-handle"/><span>边缘小把手 → 悬停或点击 → 今日安排</span></div><p>小把手会出现在屏幕{settings.sidebarSide==='left'?'左':'右'}侧。悬停片刻或点击展开，点击其他应用便收起；图钉可以固定展开。</p><button className="primary" disabled={!native} onClick={()=>void updateSettings({sidebarEnabled:true})}>{settings.sidebarEnabled?'边缘小把手已启用':'显示边缘小把手'}</button><p className="muted">关闭主窗口不会停止提醒。可从菜单栏／系统托盘重新打开；选择“退出”才停止后台运行。</p></>:<><h3>先试一次，确认提醒能看到</h3><p>只有你点击“允许通知”时才申请系统权限；点击测试后 5 秒发送，便于切到其他应用观察。</p><NotificationTest/></>}</div>
 {error?<p role="alert" className="error">{error}</p>:null}<footer><button disabled={busy} onClick={()=>void finish()}>跳过</button><div className="spacer"/>{step>0?<button disabled={busy} onClick={()=>setStep(s=>s-1)}>上一步</button>:null}<button className="primary" disabled={busy} onClick={()=>step===2?void finish():setStep(s=>s+1)}>{step===2?'开始使用':'下一步'}</button></footer></dialog>;
}
