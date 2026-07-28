const state = { data: null, wsOk: false, lastHit: null, lastEventAt: 0 };
const BOARD_ORDER = [20,1,18,4,13,6,10,15,2,17,3,19,7,16,8,11,14,9,12,5];

function $(id){ return document.getElementById(id); }
function currentPlayer(s){ return s.players.find(p => p.id === s.current_player_id) || s.players[0]; }

async function api(path, body){
  const r = await fetch(path, {method:'POST', headers:{'content-type':'application/json'}, body: body ? JSON.stringify(body) : '{}'});
  return r.json();
}
async function loadState(){ const r = await fetch('/api/state'); update({state: await r.json()}); }
function connectWs(){
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const ws = new WebSocket(`${proto}://${location.host}/ws`);
  ws.onopen = () => { state.wsOk = true; renderStatus(); };
  ws.onclose = () => { state.wsOk = false; renderStatus(); setTimeout(connectWs,1000); };
  ws.onmessage = e => update(JSON.parse(e.data));
}
function update(msg){
  if(msg.event){ state.lastEventAt = Date.now(); if(msg.event.type === 'hit' || msg.event.type === 'miss') state.lastHit = msg.event; }
  if(msg.state){ state.data = msg.state; renderAll(msg.event); }
}
function renderStatus(){ const el=$('wsStatus'); if(el) el.innerHTML=`<span class="status-dot ${state.wsOk?'ok':''}"></span>${state.wsOk?'live':'offline'}`; }
function renderAll(event){ renderStatus(); if(location.pathname.includes('projector')) renderProjector(event); else renderControl(event); }

function renderMarks(player, targets){
  if(!player.marks || !targets) return '';
  return `<div class="marks">${targets.map(t=>{const m=Math.min(3,player.marks[String(t)]||0);return `<span class="mark"><b>${t===25?'B':t}</b> ${'●'.repeat(m)}${'○'.repeat(3-m)}</span>`}).join('')}</div>`;
}
function renderPlayers(s){
  return s.players.map(p=>`<div class="player ${p.id===s.current_player_id?'active':''}"><div><b>${p.name}</b>${p.id===s.winner_id?' 🏆':''}${renderMarks(p,s.cricket_targets)}</div><div class="score">${p.score}</div></div>`).join('');
}
function holdBanner(s){ return s.status==='hold' ? '<div class="hold">Aufnahme beendet – Weiter drücken</div>' : ''; }

function renderControl(event){
  const s=state.data; if(!s)return;
  $('players').innerHTML=renderPlayers(s);
  $('meta').innerHTML=`${holdBanner(s)}<div>${s.game_type} · Status: ${s.status} · Darts: ${s.darts_in_turn}/3 · Turn: ${s.turn_score} · ${s.message||''}</div>`;
  $('last').textContent=event?JSON.stringify(event,null,2):JSON.stringify(s.last_event,null,2);
}

function ringToZone(ring){
  if(ring==='double') return 'double';
  if(ring==='triple') return 'triple';
  if(ring==='single_inner') return 'singleInner';
  if(ring==='single_outer') return 'singleOuter';
  if(ring==='single_bull') return 'singleBull';
  if(ring==='double_bull') return 'doubleBull';
  return null;
}
function polar(cx,cy,r,a){ const rad=(a-90)*Math.PI/180; return [cx+r*Math.cos(rad), cy+r*Math.sin(rad)]; }
function arcPath(cx,cy,r1,r2,a1,a2){
  const [x1,y1]=polar(cx,cy,r2,a1), [x2,y2]=polar(cx,cy,r2,a2), [x3,y3]=polar(cx,cy,r1,a2), [x4,y4]=polar(cx,cy,r1,a1);
  const large=(a2-a1)>180?1:0;
  return `M ${x1} ${y1} A ${r2} ${r2} 0 ${large} 1 ${x2} ${y2} L ${x3} ${y3} A ${r1} ${r1} 0 ${large} 0 ${x4} ${y4} Z`;
}
function boardSegmentId(zone,field){ return `seg-${zone}-${field}`; }
function buildBoard(){
  const svg=$('dartboardSvg'); if(!svg || svg.dataset.ready)return; svg.dataset.ready='1';
  const cx=250,cy=250; const rings={double:[210,235],singleOuter:[142,210],triple:[116,142],singleInner:[38,116]};
  let html=`<defs><filter id="glow"><feGaussianBlur stdDeviation="4" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter></defs>`;
  html += `<circle class="board-bg" cx="250" cy="250" r="244"/>`;
  BOARD_ORDER.forEach((field,i)=>{
    const a1=i*18-9, a2=i*18+9; const dark=i%2===0;
    for(const [zone,rr] of Object.entries(rings)){
      const cls=`seg ${zone} ${dark?'dark':'light'}`;
      html += `<path id="${boardSegmentId(zone,field)}" class="${cls}" d="${arcPath(cx,cy,rr[0],rr[1],a1,a2)}"/>`;
    }
    const [tx,ty]=polar(cx,cy,262,i*18); html += `<text class="board-num" x="${tx}" y="${ty}" text-anchor="middle" dominant-baseline="middle">${field}</text>`;
  });
  html += `<circle id="seg-singleBull-25" class="seg singleBull" cx="250" cy="250" r="38"/>`;
  html += `<circle id="seg-doubleBull-25" class="seg doubleBull" cx="250" cy="250" r="18"/>`;
  html += `<circle class="board-wire" cx="250" cy="250" r="235"/><circle class="board-wire" cx="250" cy="250" r="210"/><circle class="board-wire" cx="250" cy="250" r="142"/><circle class="board-wire" cx="250" cy="250" r="116"/><circle class="board-wire" cx="250" cy="250" r="38"/><circle class="board-wire" cx="250" cy="250" r="18"/>`;
  svg.innerHTML=html;
}
function clearBoardEffects(){ document.querySelectorAll('.seg.hit,.seg.miss-zone,.seg.target').forEach(e=>e.classList.remove('hit','miss-zone','target')); }
function renderBoardEvent(ev){
  buildBoard(); clearBoardEffects();
  const pulse=$('boardPulse'); if(pulse) pulse.className='board-pulse';
  if(!ev)return;
  if(ev.type==='miss'){
    document.querySelectorAll('.seg').forEach(e=>e.classList.add('miss-zone'));
    if(pulse) pulse.className='board-pulse miss show';
    return;
  }
  if(ev.type==='hit'){
    const zone=ringToZone(ev.ring); const field=ev.field; const id=boardSegmentId(zone,field); const el=$(id);
    if(el){ el.classList.add('hit'); }
    if(pulse) pulse.className='board-pulse hit show';
  }
}
function miniPlayers(s){
  return s.players.map(p=>`<div class="mini ${p.id===s.current_player_id?'active':''}"><span>${p.name}</span><b>${p.score}</b></div>`).join('');
}
function renderProjector(event){
  const s=state.data; if(!s)return; buildBoard();
  const p=currentPlayer(s)||{name:'-',score:'-'};
  $('game').textContent=`${s.game_type.toUpperCase()}`;
  const badge=$('statusBadge'); if(badge){ badge.textContent=s.status==='hold'?'HOLD':s.status.toUpperCase(); badge.className=`status-badge ${s.status}`; }
  $('name').textContent=p.name; $('score').textContent=p.score;
  $('turn').textContent=s.status==='hold'?'Aufnahme beendet – Weiter drücken':`Darts ${s.darts_in_turn}/3 · Turn ${s.turn_score}`;
  const ev=event||s.last_event;
  $('last').textContent=s.status==='hold'?'HOLD':(ev?(ev.label||ev.action||ev.type||''):'Ready');
  const mp=$('miniPlayers'); if(mp) mp.innerHTML=miniPlayers(s);
  renderBoardEvent(ev);
}

async function newGame(){ const names=$('names').value.split('\n').map(x=>x.trim()).filter(Boolean); await api('/api/new-game',{game_type:$('gameType').value,players:names,x01_start_score:Number($('x01').value||501)}); }
async function nextPlayer(){ await api('/api/next-player'); }
async function continueTurn(){ await api('/api/continue'); }
async function undo(){ await api('/api/undo'); }
async function inject(type,label,score,extra={}){ await api('/api/event',{type,label,score,seq:Date.now(),...extra}); }
window.addEventListener('load',()=>{ buildBoard(); loadState(); connectWs(); });
