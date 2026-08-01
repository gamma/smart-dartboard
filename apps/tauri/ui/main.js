const tauri=window.__TAURI__;
const params=new URLSearchParams(location.search);
const role=params.get('role')==='projector'?'projector':'control';
document.body.dataset.role=role;
document.querySelector('#role').textContent=role.toUpperCase();

function render(payload){
  document.querySelector('#counter').textContent=String(payload.counter ?? 0);
  document.querySelector('#status').textContent=`Runtime ${payload.runtime_instance_id} · Revision ${payload.revision}`;
  renderDisplayStatus(payload.external_display_count ?? 0);
}

function renderDisplayStatus(displayCount){
  const connected=displayCount>0;
  const status=document.querySelector('#displayStatus');
  status.dataset.connected=String(connected);
  status.textContent=connected
    ? `Projector: ${displayCount}× AirPlay / HDMI verbunden`
    : 'Projector: nicht verbunden';
}

async function start(){
  if(!tauri?.core?.invoke){
    document.querySelector('#status').textContent='Native Tauri Bridge fehlt';
    return;
  }
  render(await tauri.core.invoke('runtime_bootstrap'));
  await tauri.event.listen('runtime-state',event=>render(event.payload));
  await tauri.event.listen('display-status',event=>renderDisplayStatus(event.payload.external_display_count ?? 0));
  document.querySelector('#increment').addEventListener('click',async()=>{
    render(await tauri.core.invoke('runtime_dispatch',{action:'increment'}));
  });
}

start().catch(error=>{
  document.querySelector('#status').textContent=String(error);
});
