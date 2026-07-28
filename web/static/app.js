const state = { data: null, wsOk: false };

function $(id){ return document.getElementById(id); }
function currentPlayer(s){ return s.players.find(p => p.id === s.current_player_id) || s.players[0]; }

async function api(path, body){
  const r = await fetch(path, {
    method: 'POST',
    headers: {'content-type': 'application/json'},
    body: body ? JSON.stringify(body) : '{}'
  });
  return r.json();
}

async function loadState(){
  const r = await fetch('/api/state');
  update({state: await r.json()});
}

function connectWs(){
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const ws = new WebSocket(`${proto}://${location.host}/ws`);
  ws.onopen = () => { state.wsOk = true; renderStatus(); };
  ws.onclose = () => { state.wsOk = false; renderStatus(); setTimeout(connectWs, 1000); };
  ws.onmessage = e => update(JSON.parse(e.data));
}

function update(msg){
  if(msg.state){
    state.data = msg.state;
    renderAll(msg.event);
  }
}

function renderStatus(){
  const el = $('wsStatus');
  if(el) el.innerHTML = `<span class="status-dot ${state.wsOk ? 'ok' : ''}"></span>${state.wsOk ? 'live' : 'offline'}`;
}

function renderAll(event){
  renderStatus();
  if(location.pathname.includes('projector')) renderProjector(event);
  else renderControl(event);
}

function renderMarks(player, targets){
  if(!player.marks || !targets) return '';
  return `<div class="marks">${targets.map(t => {
    const m = Math.min(3, player.marks[String(t)] || 0);
    return `<span class="mark"><b>${t === 25 ? 'B' : t}</b> ${'●'.repeat(m)}${'○'.repeat(3-m)}</span>`;
  }).join('')}</div>`;
}

function renderPlayers(s){
  return s.players.map(p => `
    <div class="player ${p.id === s.current_player_id ? 'active' : ''}">
      <div><b>${p.name}</b>${p.id === s.winner_id ? ' 🏆' : ''}${renderMarks(p, s.cricket_targets)}</div>
      <div class="score">${p.score}</div>
    </div>
  `).join('');
}

function holdBanner(s){
  if(s.status !== 'hold') return '';
  return '<div class="hold">Aufnahme beendet – Weiter drücken</div>';
}

function renderControl(event){
  const s = state.data; if(!s) return;
  $('players').innerHTML = renderPlayers(s);
  $('meta').innerHTML = `${holdBanner(s)}<div>${s.game_type} · Status: ${s.status} · Darts: ${s.darts_in_turn}/3 · Turn: ${s.turn_score} · ${s.message || ''}</div>`;
  $('last').textContent = event ? JSON.stringify(event, null, 2) : JSON.stringify(s.last_event, null, 2);
}

function renderProjector(event){
  const s = state.data; if(!s) return;
  const p = currentPlayer(s) || {name:'-', score:'-'};
  $('game').textContent = `${s.game_type.toUpperCase()} · ${s.status}`;
  $('name').textContent = p.name;
  $('score').textContent = p.score;
  $('turn').textContent = s.status === 'hold' ? 'Aufnahme beendet – Weiter drücken' : `Darts ${s.darts_in_turn}/3 · Turn ${s.turn_score}`;
  const ev = event || s.last_event;
  $('last').textContent = s.status === 'hold' ? 'HOLD' : (ev ? (ev.label || ev.action || ev.type || '') : 'Ready');
  const playersEl = $('players');
  if(playersEl) playersEl.innerHTML = renderPlayers(s);
}

async function newGame(){
  const names = $('names').value.split('\n').map(x => x.trim()).filter(Boolean);
  await api('/api/new-game', {
    game_type: $('gameType').value,
    players: names,
    x01_start_score: Number($('x01').value || 501)
  });
}

async function nextPlayer(){ await api('/api/next-player'); }
async function continueTurn(){ await api('/api/continue'); }
async function undo(){ await api('/api/undo'); }
async function inject(type, label, score, extra={}){ await api('/api/event', {type, label, score, seq: Date.now(), ...extra}); }

window.addEventListener('load', () => { loadState(); connectWs(); });
