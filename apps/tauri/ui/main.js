const tauri=window.__TAURI__;
const params=new URLSearchParams(location.search);
const role=params.get('role')==='projector'?'projector':'control';
document.body.dataset.role=role;
document.querySelector('#role').textContent=role.toUpperCase();

function render(payload){
  document.querySelector('#counter').textContent=String(payload.counter ?? 0);
  document.querySelector('#status').textContent=`Runtime ${payload.runtime_instance_id} · Revision ${payload.revision}`;
  renderBoardStatus(payload.board);
  renderDisplayStatus(payload.external_display_count ?? 0);
  renderProjectorOutput(payload.projector_output ?? 'external_display',payload.external_display_count ?? 0,payload.counter ?? 0);
}

function renderBoardStatus(board={}){
  const labels={
    unavailable:'nicht verfügbar',permission_required:'Bluetooth erlauben',
    bluetooth_off:'Bluetooth ausgeschaltet',scanning:'sucht …',connecting:'verbindet …',
    discovering:'prüft Dienste …',subscribing:'abonniert Treffer …',ready:'bereit',
    reconnecting:'verbindet erneut …',error:'Fehler',disabled:'deaktiviert'
  };
  const status=document.querySelector('#boardStatus');
  const ready=board.phase==='ready';
  status.dataset.connected=String(ready);
  status.textContent=`Board: ${labels[board.phase] ?? board.phase ?? 'unbekannt'}`;
}

function renderDisplayStatus(displayCount){
  const connected=displayCount>0;
  const status=document.querySelector('#displayStatus');
  status.dataset.connected=String(connected);
  status.textContent=connected
    ? `Projector: ${displayCount}× AirPlay / HDMI verbunden`
    : 'Projector: nicht verbunden';
}

function renderProjectorOutput(output,displayCount,counter){
  document.body.dataset.output=output;
  for(const button of document.querySelectorAll('[data-output]')){
    button.setAttribute('aria-pressed',String(button.dataset.output===output));
  }
  const hints={
    external_display:displayCount>0?'Externes Display aktiv':'Wartet auf AirPlay- oder HDMI-Display',
    companion:'Wartet auf einen gekoppelten Companion-Projector',
    local_preview:'Lokale Vorschau aktiv'
  };
  document.querySelector('#outputHint').textContent=hints[output] ?? '';
  const preview=document.querySelector('#localPreview');
  preview.hidden=output!=='local_preview';
  document.querySelector('#previewCounter').textContent=String(counter);
}

let pairingTimer;

function renderCompanionDevices(devices=[]){
  const list=document.querySelector('#companionDevices');
  list.replaceChildren();
  if(devices.length===0){
    const empty=document.createElement('li');
    empty.className='device-empty';
    empty.textContent='Noch kein Companion gekoppelt';
    list.append(empty);
    return;
  }
  for(const device of devices){
    const item=document.createElement('li');
    const label=document.createElement('span');
    const name=document.createElement('strong');
    const roleLabel=document.createElement('small');
    const revoke=document.createElement('button');
    name.textContent=device.device_name;
    roleLabel.textContent='Projector';
    label.append(name,roleLabel);
    revoke.type='button';
    revoke.className='revoke';
    revoke.textContent='Widerrufen';
    revoke.addEventListener('click',async()=>{
      const updated=await tauri.core.invoke('companion_revoke',{deviceId:device.device_id});
      renderCompanionDevices(updated);
    });
    item.append(label,revoke);
    list.append(item);
  }
}

function renderPairingOffer(bootstrap){
  const offer=bootstrap.offer;
  const container=document.querySelector('#pairingOffer');
  const expiry=document.querySelector('#pairingExpiry');
  document.querySelector('#pairingCode').textContent=`${offer.code.slice(0,3)} ${offer.code.slice(3)}`;
  document.querySelector('#pairingFingerprint').textContent=(bootstrap.certificate_sha256.slice(0,16).match(/.{1,4}/g) ?? []).join('-').toUpperCase();
  container.hidden=false;
  clearInterval(pairingTimer);
  const update=()=>{
    const remaining=Math.max(0,offer.expires_at_ms-Date.now());
    const seconds=Math.ceil(remaining/1000);
    expiry.textContent=remaining>0
      ? `${Math.floor(seconds/60)}:${String(seconds%60).padStart(2,'0')} Minuten gültig`
      : 'Code abgelaufen';
    if(remaining===0) clearInterval(pairingTimer);
  };
  update();
  pairingTimer=setInterval(update,1000);
}

async function setupCompanions(){
  renderCompanionDevices(await tauri.core.invoke('companion_devices'));
  document.querySelector('#openPairing').addEventListener('click',async()=>{
    renderPairingOffer(await tauri.core.invoke('companion_pairing_open'));
  });
}

function setupProjectorOutput(){
  for(const button of document.querySelectorAll('[data-output]')){
    button.addEventListener('click',async()=>{
      render(await tauri.core.invoke('projector_output_select',{output:button.dataset.output}));
    });
  }
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
  if(role==='control'){
    setupProjectorOutput();
    await setupCompanions();
  }
}

start().catch(error=>{
  document.querySelector('#status').textContent=String(error);
});
