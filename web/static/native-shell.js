const tauri=window.__TAURI__;
const params=new URLSearchParams(location.search);
const role=params.get('role')==='projector'?'projector':'control';
document.body.dataset.role=role;
document.querySelector('#role').textContent=role.toUpperCase();
const backToGame=document.querySelector('#backToGame');
backToGame.hidden=role==='projector';
backToGame.addEventListener('click',()=>location.replace('/control.html'));

function render(payload){
  companionProtocolVersion=payload.companion_protocol_version ?? companionProtocolVersion;
  renderAppRole(payload.app_role ?? 'controller');
  renderProjectorValues(payload);
  document.querySelector('#status').textContent=`Runtime ${payload.runtime_instance_id} · Revision ${payload.revision}`;
  renderBoardStatus(payload.board);
  renderDisplayStatus(payload.external_display_count ?? 0);
  renderProjectorOutput(payload.projector_output ?? 'external_display',payload.external_display_count ?? 0,payload.counter ?? 0,payload.companion_port ?? null,payload.companion_available ?? false);
}

function renderProjectorValues(payload){
  const counter=String(payload.counter ?? 0);
  document.querySelector('#counter').textContent=counter;
  document.querySelector('#previewCounter').textContent=counter;
  document.querySelector('#companionCounter').textContent=counter;
}

function renderCompanionFrame(payload){
  renderProjectorValues(payload);
  document.querySelector('#companionRuntimeStatus').textContent=`Runtime ${payload.runtime_instance_id} · Revision ${payload.revision}`;
}

let currentAppRole='controller';
let companionProtocolVersion=2;
let discoveryTimer;
let currentPairingTarget;
let currentCompanionClientPhase;

function renderAppRole(appRole){
  currentAppRole=appRole;
  document.body.dataset.appRole=appRole;
  for(const button of document.querySelectorAll('[data-app-role]')){
    button.setAttribute('aria-pressed',String(button.dataset.appRole===appRole));
  }
  document.querySelector('#companionProjectorView').hidden=appRole!=='companion_projector';
  backToGame.hidden=role==='projector'||appRole==='companion_projector';
  document.querySelector('#roleError').textContent='';
}

function renderDiscoveredHosts(hosts=[]){
  const list=document.querySelector('#discoveredHosts');
  list.replaceChildren();
  const status=document.querySelector('#discoveryStatus');
  if(!currentCompanionClientPhase){
    status.dataset.connected=String(hosts.length>0);
    status.textContent=hosts.length>0
      ? `${hosts.length} Controller gefunden`
      : 'Lokales Netzwerk wird durchsucht …';
  }
  for(const host of hosts){
    const item=document.createElement('li');
    const copy=document.createElement('div');
    const name=document.createElement('strong');
    const details=document.createElement('span');
    const select=document.createElement('button');
    const compatible=host.protocol_version===companionProtocolVersion&&host.tls===true;
    name.textContent=host.service_name;
    details.textContent=compatible
      ? `${host.host_name} · sicherer Dienst`
      : `Nicht kompatible Protokollversion ${host.protocol_version}`;
    if(!compatible) item.className='incompatible';
    copy.append(name,details);
    select.type='button';
    select.textContent='Auswählen';
    select.disabled=!compatible;
    select.addEventListener('click',()=>prepareClientPairing(host));
    item.append(copy,select);
    list.append(item);
  }
}

async function prepareClientPairing(host){
  const status=document.querySelector('#clientPairingStatus');
  status.textContent='TLS-Zertifikat wird gelesen …';
  document.querySelector('#companionClientPairing').hidden=false;
  try{
    currentPairingTarget=await tauri.core.invoke('companion_pairing_prepare',{hostId:host.host_id});
    document.querySelector('#clientPairingName').textContent=currentPairingTarget.service_name;
    document.querySelector('#clientPairingFingerprint').textContent=currentPairingTarget.manual_fingerprint;
    document.querySelector('#fingerprintConfirmed').checked=false;
    document.querySelector('#clientPairingCode').value='';
    status.textContent='Vergleichen, bestätigen und den Code vom Controller eingeben.';
    updateCompletePairingButton();
  }catch(error){
    currentPairingTarget=undefined;
    status.textContent=String(error);
  }
}

function updateCompletePairingButton(){
  const confirmed=document.querySelector('#fingerprintConfirmed').checked;
  const code=document.querySelector('#clientPairingCode').value.replace(/\D/g,'');
  document.querySelector('#completeClientPairing').disabled=!currentPairingTarget||!confirmed||code.length!==6;
}

function setupCompanionClient(){
  const code=document.querySelector('#clientPairingCode');
  code.addEventListener('input',()=>{
    code.value=code.value.replace(/\D/g,'').slice(0,6);
    updateCompletePairingButton();
  });
  document.querySelector('#fingerprintConfirmed').addEventListener('change',updateCompletePairingButton);
  document.querySelector('#completeClientPairing').addEventListener('click',async()=>{
    const button=document.querySelector('#completeClientPairing');
    const status=document.querySelector('#clientPairingStatus');
    button.disabled=true;
    status.textContent='Sichere Kopplung läuft …';
    try{
      const paired=await tauri.core.invoke('companion_pairing_complete',{
        hostId:currentPairingTarget.host_id,
        manualFingerprint:currentPairingTarget.manual_fingerprint,
        code:code.value
      });
      renderCompanionClientStatus(paired);
      status.textContent=`Mit ${paired.service_name} gekoppelt. Projector wird verbunden …`;
    }catch(error){
      status.textContent=String(error);
      updateCompletePairingButton();
    }
  });
  updateCompletePairingButton();
}

function renderCompanionClientStatus(status){
  const setup=document.querySelector('#companionConnectionSetup');
  const stage=document.querySelector('#companionProjectorStage');
  const discovery=document.querySelector('#discoveryStatus');
  const connected=status?.phase==='connected';
  setup.hidden=connected;
  stage.hidden=!connected;
  if(!status){
    currentCompanionClientPhase=undefined;
    return;
  }
  currentCompanionClientPhase=status.phase;
  const labels={
    discovering:'Gekoppelter Controller wird gesucht …',
    connecting:`Verbindung zu ${status.service_name} wird hergestellt …`,
    reconnecting:'Verbindung unterbrochen · neuer Snapshot wird angefordert …',
    pairing_required:'Kopplung wurde widerrufen · Controller erneut auswählen',
    connected:`Mit ${status.service_name} verbunden`
  };
  discovery.dataset.connected=String(connected);
  discovery.textContent=labels[status.phase] ?? 'Companion wird vorbereitet …';
  if(connected){
    document.querySelector('#companionRuntimeStatus').textContent=`Runtime ${status.runtime_instance_id} · Revision ${status.revision}`;
    location.replace(new URL('./projector.html?native-companion=1',location.href));
  }
}

async function pollDiscovery(){
  if(currentAppRole!=='companion_projector') return;
  try{
    renderDiscoveredHosts(await tauri.core.invoke('companion_discovered_hosts'));
  }catch(error){
    const status=document.querySelector('#discoveryStatus');
    status.dataset.connected='false';
    status.textContent=String(error);
  }
}

async function startDiscovery(){
  clearInterval(discoveryTimer);
  await tauri.core.invoke('companion_discovery_start');
  await pollDiscovery();
  discoveryTimer=setInterval(pollDiscovery,1000);
}

async function stopDiscovery(){
  clearInterval(discoveryTimer);
  discoveryTimer=undefined;
  await tauri.core.invoke('companion_discovery_stop');
}

async function applyRoleLifecycle(){
  if(currentAppRole==='companion_projector'){
    await startDiscovery();
    renderCompanionClientStatus(await tauri.core.invoke('companion_client_status'));
  }else{
    await stopDiscovery();
    renderCompanionDevices(await tauri.core.invoke('companion_devices'));
    currentPairingTarget=undefined;
    document.querySelector('#companionClientPairing').hidden=true;
  }
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

function renderProjectorOutput(output,displayCount,counter,companionPort,companionAvailable){
  document.body.dataset.output=output;
  for(const button of document.querySelectorAll('[data-output]')){
    button.setAttribute('aria-pressed',String(button.dataset.output===output));
    if(button.dataset.output==='companion') button.disabled=!companionAvailable;
  }
  const hints={
    external_display:displayCount>0?'Externes Display aktiv':'Wartet auf AirPlay- oder HDMI-Display',
    companion:!companionAvailable?'Companion ist auf diesem Gerät derzeit nicht verfügbar':companionPort?`Sicherer Companion-Dienst bereit · Port ${companionPort}`:'Sicherer Companion-Dienst startet …',
    local_preview:'Lokale Vorschau aktiv'
  };
  document.querySelector('#outputHint').textContent=hints[output] ?? '';
  const preview=document.querySelector('#localPreview');
  preview.hidden=output!=='local_preview';
  document.querySelector('#previewCounter').textContent=String(counter);
  const pairingButton=document.querySelector('#openPairing');
  if(pairingButton) pairingButton.disabled=output!=='companion'||!companionPort;
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
  document.querySelector('#openPairing').addEventListener('click',async()=>{
    renderPairingOffer(await tauri.core.invoke('companion_pairing_open'));
  });
}

function setupAppRole(){
  for(const button of document.querySelectorAll('[data-app-role]')){
    button.addEventListener('click',async()=>{
      if(button.dataset.appRole===currentAppRole) return;
      for(const roleButton of document.querySelectorAll('[data-app-role]')) roleButton.disabled=true;
      try{
        render(await tauri.core.invoke('app_role_select',{role:button.dataset.appRole}));
        await applyRoleLifecycle();
      }catch(error){
        document.querySelector('#roleError').textContent=String(error);
      }finally{
        for(const roleButton of document.querySelectorAll('[data-app-role]')) roleButton.disabled=false;
      }
    });
  }
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
  const initial=await tauri.core.invoke('runtime_bootstrap');
  render(initial);
  await tauri.event.listen('runtime-state',event=>render(event.payload));
  await tauri.event.listen('display-status',event=>renderDisplayStatus(event.payload.external_display_count ?? 0));
  await tauri.event.listen('companion-projector-status',event=>renderCompanionClientStatus(event.payload));
  await tauri.event.listen('companion-projector-frame',event=>renderCompanionFrame(event.payload));
  document.querySelector('#increment').addEventListener('click',async()=>{
    render(await tauri.core.invoke('runtime_dispatch',{action:'increment'}));
  });
  if(role==='control'){
    setupAppRole();
    setupCompanionClient();
    setupProjectorOutput();
    await setupCompanions();
    await applyRoleLifecycle();
  }
}

start().catch(error=>{
  document.querySelector('#status').textContent=String(error);
});
