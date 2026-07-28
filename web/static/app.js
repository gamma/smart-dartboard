const appState = {
  experience: null,
  wsOk: false,
  selectedPlayers: new Set(),
  audio: null,
  countdownTimer: null,
  lastCueKey: null,
  projectedEvent: null,
  boardResetTimer: null,
  selectedCorrectionIndex: null,
};

const BOARD_ORDER = [20,1,18,4,13,6,10,15,2,17,3,19,7,16,8,11,14,9,12,5];
const BOARD_EVENT_VISIBLE_MS = 4000;
const AVATARS = [
  {id:'comet', emoji:'☄️', label:'Komet'},
  {id:'nova', emoji:'🌟', label:'Stern'},
  {id:'bolt', emoji:'⚡', label:'Blitz'},
  {id:'viper', emoji:'🐍', label:'Schlange'},
  {id:'orbit', emoji:'🪐', label:'Planet'},
  {id:'crown', emoji:'👑', label:'Krone'},
  {id:'bull', emoji:'🎯', label:'Ziel'},
  {id:'fire', emoji:'🔥', label:'Feuer'},
  {id:'unicorn', emoji:'🦄', label:'Einhorn'},
  {id:'ninja', emoji:'🥷', label:'Ninja'},
  {id:'robot', emoji:'🤖', label:'Roboter'},
  {id:'party', emoji:'🥳', label:'Party'},
];
const COLORS = ['#28e7ff','#ffb52b','#3dff91','#ff4f79','#a77bff','#ffffff'];

function $(id){ return document.getElementById(id); }
function isProjector(){ return location.pathname.includes('projector'); }
function escapeHtml(value){
  return String(value ?? '').replace(/[&<>"']/g, char => ({
    '&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#039;'
  })[char]);
}
function modeBySlug(slug){
  return appState.experience?.modes.find(mode => mode.slug === slug);
}
function modeAsset(slug){
  return `/static/assets/modes/${encodeURIComponent(slug)}.webp`;
}
function avatarEmoji(avatar){
  return AVATARS.find(option => option.id === avatar)?.emoji || '🎯';
}
function currentPlayer(game){
  return game?.players.find(player => player.id === game.current_player_id) || game?.players[0];
}
function testModeEnabled(){
  return Boolean(appState.experience?.hardware?.test_events);
}

async function api(path, body = {}){
  const response = await fetch(path, {
    method: 'POST',
    headers: {'content-type':'application/json'},
    body: JSON.stringify(body),
  });
  const payload = await response.json();
  if(!response.ok) throw new Error(payload.detail || `HTTP ${response.status}`);
  return payload;
}
async function action(path, body = {}){
  try { return await api(path, body); }
  catch(error){ showToast(error.message); throw error; }
}
function showToast(message){
  const toast = $('toast');
  if(!toast) return;
  toast.textContent = message;
  toast.classList.add('show');
  setTimeout(() => toast.classList.remove('show'), 2600);
}

async function loadBootstrap(){
  const response = await fetch('/api/bootstrap');
  updateExperience(await response.json());
}
function connectWs(){
  const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
  const ws = new WebSocket(`${protocol}://${location.host}/ws`);
  ws.onopen = () => { appState.wsOk = true; renderConnection(); };
  ws.onclose = () => {
    appState.wsOk = false;
    renderConnection();
    setTimeout(connectWs, 1000);
  };
  ws.onmessage = message => {
    const payload = JSON.parse(message.data);
    if(payload.experience) updateExperience(payload.experience, payload.event);
  };
}
function updateExperience(experience, event){
  const previous = appState.experience;
  appState.experience = experience;
  if(event){
    const isThrow=event.type==='hit' || event.type==='miss';
    const lastThrow=experience.game?.throws?.at(-1);
    const throwWasCounted=!isThrow || (lastThrow && Number(lastThrow.seq)===Number(event.seq));
    if(throwWasCounted) playEventCue(event, experience);
    if(isThrow && throwWasCounted){
      appState.selectedCorrectionIndex=null;
      appState.projectedEvent=event;
      clearTimeout(appState.boardResetTimer);
      appState.boardResetTimer=setTimeout(()=>{
        appState.projectedEvent=null;
        if(isProjector() && appState.experience?.screen==='playing') renderProjector();
      },BOARD_EVENT_VISIBLE_MS);
    }
    if(event.type==='throw_corrected') appState.selectedCorrectionIndex=null;
  }
  if(previous?.screen !== experience.screen && experience.screen === 'countdown'){
    startCountdown();
  }
  render();
}
function renderConnection(){
  const element = $('wsStatus');
  if(!element) return;
  const hardware=appState.experience?.hardware;
  const boardReady=!hardware?.enabled || hardware.status==='connected';
  const cssClass=!appState.wsOk?'':boardReady?'online':'searching';
  const label=!appState.wsOk?'OFFLINE':boardReady?'LIVE':hardware?.status==='error'?'BOARD FEHLER':'BOARD SUCHT';
  element.innerHTML = `<i class="${cssClass}"></i>${label}`;
}
function render(){
  renderConnection();
  if(!appState.experience) return;
  if(isProjector()) renderProjector();
  else renderControl();
}

function sceneHeader(kicker, title, copy = ''){
  return `<header class="scene-header">
    <div class="kicker">${escapeHtml(kicker)}</div>
    <h1>${escapeHtml(title)}</h1>
    ${copy ? `<p>${escapeHtml(copy)}</p>` : ''}
  </header>`;
}
function actionButton(label, actionName, kind = 'primary', extra = ''){
  return `<button class="action-button ${kind}" data-action="${actionName}" ${extra}>${label}</button>`;
}

function renderControl(){
  const root = $('controlRoot');
  if(!root) return;
  const screen = appState.experience.screen;
  const renderers = {
    attract: controlAttract,
    players: controlPlayers,
    game_select: controlGameSelect,
    instructions: controlInstructions,
    countdown: controlCountdown,
    playing: controlPlaying,
    game_result: controlGameResult,
    session_summary: controlSessionSummary,
    calibration: controlCalibration,
  };
  root.innerHTML = (renderers[screen] || controlAttract)();
  if(screen==='playing' && $('dartboardSvg')) buildBoard();
}

function controlAttract(){
  return `<section class="control-scene attract-control">
    <div class="hero-copy">
      <div class="kicker">READY WHEN YOU ARE</div>
      <h1>Deine Darts.<br><span>Deine Session.</span></h1>
      <p>Spieler auswählen, Modus antippen und loslegen.</p>
      <div class="button-row">
        ${actionButton('Session starten', 'choose-players')}
        ${actionButton('Projektor kalibrieren', 'calibrate', 'ghost')}
      </div>
    </div>
    <div class="hero-orbit" aria-hidden="true"><i></i><i></i><i></i><b>◎</b></div>
  </section>`;
}

function playerCard(player, selected){
  return `<button class="player-select ${selected ? 'selected' : ''}" data-action="toggle-player" data-id="${escapeHtml(player.id)}">
    <span class="avatar avatar-${escapeHtml(player.avatar)}" style="--player:${escapeHtml(player.color)}" aria-label="${escapeHtml(player.avatar)}">${avatarEmoji(player.avatar)}</span>
    <span><b>${escapeHtml(player.name)}</b><small>${selected ? 'Ausgewählt' : 'Antippen'}</small></span>
    <i>${selected ? '✓' : '+'}</i>
  </button>`;
}
function controlPlayers(){
  const players = appState.experience.players;
  const cards = players.map(player => playerCard(player, appState.selectedPlayers.has(player.id))).join('');
  return `<section class="control-scene">
    ${sceneHeader('SCHRITT 1 VON 2','Wer spielt heute?','Wähle bis zu acht Spieler für diese Session.')}
    <div class="player-layout">
      <div class="selection-grid">${cards || '<div class="empty-state">Lege deinen ersten Spieler an.</div>'}</div>
      <form id="newPlayerForm" class="new-player-card">
        <h2>Neuer Spieler</h2>
        <label>Name<input name="name" maxlength="32" autocomplete="off" placeholder="Spielername" required></label>
        <div class="choice-label">Avatar</div>
        <div class="avatar-choices">${AVATARS.map((avatar,index) =>
          `<label class="mini-choice" title="${escapeHtml(avatar.label)}"><input type="radio" name="avatar" value="${escapeHtml(avatar.id)}" ${index===0?'checked':''}><span>${avatar.emoji}</span></label>`
        ).join('')}</div>
        <div class="choice-label">Farbe</div>
        <div class="color-choices">${COLORS.map((color,index) =>
          `<label><input type="radio" name="color" value="${color}" ${index===0?'checked':''}><span style="--swatch:${color}"></span></label>`
        ).join('')}</div>
        ${actionButton('Spieler anlegen','create-player','secondary','type="submit"')}
      </form>
    </div>
    <footer class="sticky-actions">
      ${actionButton('Zurück','home','ghost')}
      <div><b>${appState.selectedPlayers.size}</b> Spieler gewählt</div>
      ${actionButton('Weiter zur Spielwahl','start-session','primary',appState.selectedPlayers.size ? '' : 'disabled')}
    </footer>
  </section>`;
}

function modeCard(mode){
  return `<button class="mode-card" data-action="select-mode" data-mode="${escapeHtml(mode.slug)}" style="--accent:${escapeHtml(mode.accent)}">
    <img src="${modeAsset(mode.slug)}" alt="" loading="eager">
    <span class="mode-shade"></span>
    <span class="mode-copy"><small>${escapeHtml(mode.tagline)}</small><b>${escapeHtml(mode.title)}</b><em>${escapeHtml(mode.description)}</em></span>
    <i>→</i>
  </button>`;
}
function controlGameSelect(){
  const session = appState.experience.session;
  return `<section class="control-scene">
    ${sceneHeader('SCHRITT 2 VON 2','Wählt euer Spiel',`${session?.players.length || 0} Spieler · alle Modi sind sofort startklar.`)}
    <div class="mode-grid">${appState.experience.modes.map(modeCard).join('')}</div>
    <footer class="sticky-actions">
      ${actionButton('Session beenden','end-session','ghost')}
      <div class="session-people">${(session?.players || []).map(player => `<span style="--player:${escapeHtml(player.color)}">${escapeHtml(player.name)}</span>`).join('')}</div>
    </footer>
  </section>`;
}

function optionControl(option, selected){
  return `<div class="option-block"><div class="choice-label">${escapeHtml(option.label)}</div><div class="segmented">
    ${option.choices.map(choice => `<button class="${String(choice.value)===String(selected) ? 'selected' : ''}" data-action="set-option" data-key="${escapeHtml(option.key)}" data-value="${escapeHtml(choice.value)}">${escapeHtml(choice.label)}</button>`).join('')}
  </div></div>`;
}
function instructionSteps(mode){
  return mode.instructions.map((step,index) => `<article class="instruction-step">
    <span>${index+1}</span><div><h3>${escapeHtml(step.title)}</h3><p>${escapeHtml(step.body)}</p></div>
  </article>`).join('');
}
function controlInstructions(){
  const mode = modeBySlug(appState.experience.selected_mode);
  if(!mode) return controlGameSelect();
  return `<section class="control-scene instruction-control" style="--accent:${escapeHtml(mode.accent)};--hero:url('${modeAsset(mode.slug)}')">
    <div class="instruction-hero">
      <div>${sceneHeader(mode.tagline,mode.title,mode.description)}</div>
    </div>
    <div class="instruction-content">
      <div class="instruction-list">${instructionSteps(mode)}</div>
      <div class="game-options">${mode.options.map(option => optionControl(option,appState.experience.selected_options[option.key])).join('')}</div>
    </div>
    <footer class="sticky-actions">
      ${actionButton('Andere Spielart','back-games','ghost')}
      ${actionButton('Spiel starten','start-game')}
    </footer>
  </section>`;
}

function controlCountdown(){
  return `<section class="control-scene centered-scene">
    <div class="radar-loader"><i></i><b>3</b></div>
    <div class="kicker">PROJEKTOR LÄUFT</div>
    <h1>Spiel startet …</h1>
    <p>Alle Spieler bereit an die Linie.</p>
  </section>`;
}

function marks(player, targets){
  if(!player.marks || !targets) return '';
  return `<div class="marks">${targets.map(target => {
    const count = Math.min(3, player.marks[String(target)] || 0);
    return `<span><b>${target===25?'B':target}</b><i>${'●'.repeat(count)}${'○'.repeat(3-count)}</i></span>`;
  }).join('')}</div>`;
}
function scoreboard(game){
  return game.players.map(player => `<article class="score-row ${player.id===game.current_player_id?'active':''}" style="--player:${escapeHtml(player.color)}">
    <span class="avatar avatar-${escapeHtml(player.avatar)}" aria-label="${escapeHtml(player.avatar)}">${avatarEmoji(player.avatar)}</span>
    <div><b>${escapeHtml(player.name)}</b>${game.game_type==='cricket' ? marks(player,game.cricket_targets) : ''}</div>
    <strong>${player.score}</strong>
  </article>`).join('');
}
function currentTurnThrows(game){
  return game.darts_in_turn > 0 ? game.throws.slice(-game.darts_in_turn) : [];
}
function turnDartCards(game){
  const throws=currentTurnThrows(game);
  return [0,1,2].map(index=>{
    const dart=throws[index];
    if(!dart){
      return `<div class="turn-dart empty"><span>${index+1}</span><b>—</b><small>NOCH OFFEN</small></div>`;
    }
    return `<button class="turn-dart ${appState.selectedCorrectionIndex===index?'selected':''}" data-action="select-correction" data-index="${index}">
      <span>${index+1}</span><b>${escapeHtml(dart.label || 'MISS')}</b><small>${dart.score} PUNKTE · ANTIPPEN ZUM KORRIGIEREN</small>
    </button>`;
  }).join('');
}
function correctionPanel(){
  const selected=appState.selectedCorrectionIndex;
  if(selected===null){
    return '<div class="correction-hint">Zum Korrigieren einen der aktuellen Würfe antippen.</div>';
  }
  return `<section class="correction-panel">
    <div class="correction-board-shell">
      <svg id="dartboardSvg" class="dartboard-svg correction-board" viewBox="0 0 500 500" aria-label="Korrektur-Dartboard"></svg>
    </div>
    <div class="correction-copy">
      <div class="kicker">WURF ${selected+1} KORRIGIEREN</div>
      <h2>Neues Feld antippen</h2>
      <p>Wähle den tatsächlich getroffenen Bereich direkt auf der Scheibe. Der aktuelle Spielstand und alle folgenden Würfe werden automatisch neu berechnet.</p>
      <div class="correction-actions">
        ${actionButton('Als MISS werten','correct-miss','danger')}
        ${actionButton('Abbrechen','cancel-correction','ghost')}
      </div>
    </div>
  </section>`;
}
function x01AdvicePanel(game){
  const advice = game.advice;
  if(game.game_type !== 'x01' || !advice || !advice.primary) return '';
  const sequence = (advice.sequence || []).map(dart => `<b>${escapeHtml(dart.label)}</b>`).join('<span>→</span>');
  const follow = advice.setup?.remaining_checkout?.length ? advice.setup.remaining_checkout.map(dart => dart.label).join(' → ') : '';
  const title = advice.status === 'checkout' ? 'Finish möglich' : advice.status === 'setup' ? 'Clever stellen' : 'Runterspielen';
  return `<aside class="x01-advice ${escapeHtml(advice.status)}">
    <div><span class="kicker">X01 ADVISOR · ${advice.darts_left} DARTS</span><h2>${title}: ${escapeHtml(advice.primary.label)}</h2><p>${escapeHtml(advice.message || '')}</p></div>
    ${sequence ? `<div class="checkout-sequence">${sequence}</div>` : ''}
    ${follow ? `<small>Danach: ${escapeHtml(follow)}</small>` : ''}
  </aside>`;
}

function controlPlaying(){
  const game = appState.experience.game;
  const mode = modeBySlug(game.game_type);
  return `<section class="control-scene play-control" style="--accent:${escapeHtml(mode?.accent || '#28e7ff')}">
    <div class="play-heading">
    <div><div class="kicker">${escapeHtml(mode?.title || game.game_type)} · RUNDE ${game.round_number}</div><h1>${game.status==='hold'?'Aufnahme beendet':`${escapeHtml(currentPlayer(game)?.name || '')} ist dran`}</h1></div>
      <div class="turn-counter"><span>${game.darts_in_turn}</span><small>/ 3 DARTS</small><b>${game.turn_score} PTS</b></div>
    </div>
    ${x01AdvicePanel(game)}
    <div class="turn-darts">${turnDartCards(game)}</div>
    ${correctionPanel()}
    <div class="scoreboard">${scoreboard(game)}</div>
    <div class="operator-panel">
      ${game.status==='hold' ? actionButton('Weiter zum nächsten Spieler','continue','primary') : actionButton('Spieler wechseln','next-player','secondary')}
      ${actionButton('Letzten Wurf zurück','undo','danger')}
    </div>
  </section>`;
}

function winner(){
  const game = appState.experience.game;
  return game.players.find(player => player.id === game.winner_id);
}
function controlGameResult(){
  const champion = winner();
  return `<section class="control-scene result-control">
    <div class="trophy-orbit">♛</div>
    <div class="kicker">GAME COMPLETE</div>
    <h1>${escapeHtml(champion?.name || 'Spiel beendet')}</h1>
    <p>${champion ? 'holt sich den Sieg.' : 'Das Spiel ist abgeschlossen.'}</p>
    <div class="result-actions">
      ${actionButton('Nächstes Spiel','next-game')}
      ${actionButton('Session abschließen','end-session','ghost')}
    </div>
  </section>`;
}
function statCards(){
  return appState.experience.statistics.map(stat => `<article class="stat-card" style="--player:${escapeHtml(stat.color)}">
    <h3>${escapeHtml(stat.name)}</h3>
    <div><span><b>${stat.wins}</b><small>Siege</small></span><span><b>${stat.three_dart_average}</b><small>3-Dart Ø</small></span><span><b>${stat.best_dart}</b><small>Best Dart</small></span><span><b>${stat.total_points}</b><small>Punkte</small></span></div>
  </article>`).join('');
}
function controlSessionSummary(){
  return `<section class="control-scene">
    ${sceneHeader('SESSION COMPLETE','Eure Highlights','Alle Ergebnisse wurden dauerhaft gespeichert.')}
    <div class="stats-grid">${statCards()}</div>
    <footer class="sticky-actions">
      ${actionButton('Zur Startseite','close-session')}
    </footer>
  </section>`;
}

function cornerControls(calibration){
  const labels = ['Oben links','Oben rechts','Unten rechts','Unten links'];
  return calibration.corners.map((corner,index) => `<fieldset class="corner-control"><legend>${labels[index]}</legend>
    <label>X <input type="range" min="0" max="1" step="0.002" value="${corner.x}" data-corner="${index}" data-axis="x"><output>${Math.round(corner.x*100)}%</output></label>
    <label>Y <input type="range" min="0" max="1" step="0.002" value="${corner.y}" data-corner="${index}" data-axis="y"><output>${Math.round(corner.y*100)}%</output></label>
  </fieldset>`).join('');
}
function controlCalibration(){
  return `<section class="control-scene calibration-control">
    ${sceneHeader('EINMALIGES SETUP','Projektor ausrichten','Verschiebe die vier Eckpunkte, bis der äußere Ring exakt auf der echten Scheibe liegt.')}
    <div class="calibration-grid">${cornerControls(appState.experience.calibration)}</div>
    <p class="calibration-note">Die Perspektive wird als Softwareprofil gespeichert. Der Projektor muss danach im Alltag nicht mehr nachjustiert werden.</p>
    <footer class="sticky-actions">
      ${actionButton('Abbrechen','home','ghost')}
      ${actionButton('Kalibrierung speichern','save-calibration')}
    </footer>
  </section>`;
}

function renderProjector(){
  const root = $('projectorRoot');
  if(!root) return;
  const screen = appState.experience.screen;
  const renderers = {
    attract: projectorAttract,
    players: projectorPlayers,
    game_select: projectorGameSelect,
    instructions: projectorInstructions,
    countdown: projectorCountdown,
    playing: projectorPlaying,
    game_result: projectorResult,
    session_summary: projectorSummary,
    calibration: projectorCalibration,
  };
  root.innerHTML = (renderers[screen] || projectorAttract)();
  if(screen === 'playing' || screen === 'calibration'){
    buildBoard();
    applyCalibration();
    if(screen === 'playing') renderBoardEvent(appState.projectedEvent);
  }
}
function projectorBackdrop(mode, inner, className=''){
  const image = modeAsset(mode?.slug || 'countup');
  return `<section class="projector-scene ${className}" style="--scene-image:url('${image}');--accent:${escapeHtml(mode?.accent || '#28e7ff')}"><div class="cinema-shade"></div>${inner}</section>`;
}
function projectorAttract(){
  return projectorBackdrop(null,`<div class="projector-center"><div class="projector-logo">◎</div><div class="kicker">SMART DART EXPERIENCE</div><h1>Bereit für<br><span>eure Session?</span></h1><p>Am Control-Screen starten</p></div>`,'attract-projector');
}
function projectorPlayers(){
  return projectorBackdrop(null,`<div class="projector-center"><div class="kicker">SESSION SETUP</div><h1>Wer spielt heute?</h1><p>Wählt eure Spieler am Control-Screen.</p></div>`);
}
function projectorGameSelect(){
  const players = appState.experience.session?.players || [];
  return projectorBackdrop(null,`<div class="projector-center"><div class="kicker">TEAM READY</div><h1>${players.map(player=>escapeHtml(player.name)).join(' · ')}</h1><p>Wählt jetzt euren Spielmodus.</p></div>`);
}
function projectorInstructions(){
  const mode = modeBySlug(appState.experience.selected_mode);
  return projectorBackdrop(mode,`<div class="projector-instructions">
    <div><div class="kicker">${escapeHtml(mode.tagline)}</div><h1>${escapeHtml(mode.title)}</h1><p>${escapeHtml(mode.description)}</p></div>
    <div class="projector-step-list">${instructionSteps(mode)}</div>
  </div>`,'mode-projector');
}
function projectorCountdown(){
  const mode = modeBySlug(appState.experience.selected_mode);
  return projectorBackdrop(mode,`<div class="projector-center"><div id="projectorCountdown" class="giant-countdown">3</div><div class="kicker">GET READY</div></div>`,'countdown-projector');
}

function boardSvg(){
  return `<svg id="dartboardSvg" class="dartboard-svg" viewBox="0 0 500 500" aria-label="Kalibrierte Dartboard-Projektion"></svg>`;
}
function projectorAdvice(game){
  const advice = game.advice;
  if(game.game_type !== 'x01' || !advice || !advice.primary || game.status === 'hold') return '';
  const headline = advice.status === 'checkout' ? 'FINISH' : advice.status === 'setup' ? 'STELLEN' : 'NÄCHSTER WURF';
  const sequence = (advice.sequence || []).map(dart => dart.label).join(' → ');
  return `<aside class="projector-advice ${escapeHtml(advice.status)}"><span>${headline}</span><b>${escapeHtml(advice.primary.label)}</b><small>${escapeHtml(sequence || advice.message || '')}</small></aside>`;
}

function projectorPlaying(){
  const game = appState.experience.game;
  const mode = modeBySlug(game.game_type);
  const player = currentPlayer(game) || {};
  const testMode=testModeEnabled();
  return `<section class="projection-game ${testMode?'test-mode':''}" style="--accent:${escapeHtml(mode?.accent || '#28e7ff')}">
    <div id="projectionPlane" class="projection-plane">${boardSvg()}<div id="boardPulse" class="board-pulse"></div></div>
    <header class="projection-top"><div><div class="kicker">${escapeHtml(mode?.title || '')} · RUNDE ${game.round_number}</div><h1>${escapeHtml(player.name || '')}</h1></div><strong>${player.score ?? 0}</strong></header>
    <footer class="projection-bottom">
      <div class="throw-callout">${game.status==='hold'?'DARTS ZIEHEN':escapeHtml(appState.projectedEvent?.label || 'BEREIT')}</div>
      <div>${game.darts_in_turn}/3 DARTS · ${game.turn_score} PTS</div>
    </footer>
    ${projectorAdvice(game)}
    <aside class="projection-roster">${game.players.map(item=>`<span class="${item.id===game.current_player_id?'active':''}"><b>${escapeHtml(item.name)}</b><i>${item.score}</i></span>`).join('')}</aside>
    ${testMode?'<div class="projector-test-tools"><b>TESTMODUS</b><span>Scheibensegment anklicken</span><button data-action="test-miss">MISS</button></div>':''}
  </section>`;
}
function projectorResult(){
  const champion = winner();
  const mode = modeBySlug(appState.experience.game.game_type);
  return projectorBackdrop(mode,`<div class="projector-center winner-scene"><div class="winner-crown">♛</div><div class="kicker">WINNER</div><h1>${escapeHtml(champion?.name || 'GAME OVER')}</h1><p class="result-score">${champion?.score ?? ''} PUNKTE</p></div>`,'result-projector');
}
function projectorSummary(){
  return projectorBackdrop(null,`<div class="projector-summary"><div>${sceneHeader('SESSION COMPLETE','Was für eine Runde!','Eure Highlights')}</div><div class="projector-stats">${statCards()}</div></div>`);
}
function projectorCalibration(){
  return `<section class="projection-game calibration-projector">
    <div id="projectionPlane" class="projection-plane calibration-plane">${boardSvg()}<span class="calibration-cross">+</span></div>
    <div class="calibration-caption"><b>KALIBRIERUNG</b><span>Äußeren Ring am Control-Screen deckungsgleich ausrichten</span></div>
  </section>`;
}

function polar(cx,cy,r,angle){
  const radians = (angle-90)*Math.PI/180;
  return [cx+r*Math.cos(radians),cy+r*Math.sin(radians)];
}
function arcPath(cx,cy,r1,r2,a1,a2){
  const [x1,y1]=polar(cx,cy,r2,a1), [x2,y2]=polar(cx,cy,r2,a2);
  const [x3,y3]=polar(cx,cy,r1,a2), [x4,y4]=polar(cx,cy,r1,a1);
  return `M ${x1} ${y1} A ${r2} ${r2} 0 0 1 ${x2} ${y2} L ${x3} ${y3} A ${r1} ${r1} 0 0 0 ${x4} ${y4} Z`;
}
function boardSegmentId(zone,field){ return `seg-${zone}-${field}`; }
function buildBoard(){
  const svg = $('dartboardSvg');
  if(!svg || svg.dataset.ready) return;
  svg.dataset.ready='1';
  const rings={double:[210,235],singleOuter:[142,210],triple:[116,142],singleInner:[38,116]};
  let html='<defs><filter id="glow"><feGaussianBlur stdDeviation="4" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter></defs><circle class="board-bg" cx="250" cy="250" r="244"/>';
  BOARD_ORDER.forEach((field,index)=>{
    const a1=index*18-9, a2=index*18+9;
    for(const [zone,radii] of Object.entries(rings)){
      html += `<path id="${boardSegmentId(zone,field)}" class="seg ${zone} ${index%2===0?'dark':'light'}" d="${arcPath(250,250,radii[0],radii[1],a1,a2)}"/>`;
    }
    const [x,y]=polar(250,250,263,index*18);
    html += `<text class="board-num" x="${x}" y="${y}" text-anchor="middle" dominant-baseline="middle">${field}</text>`;
  });
  html += '<circle id="seg-singleBull-25" class="seg singleBull" cx="250" cy="250" r="38"/><circle id="seg-doubleBull-25" class="seg doubleBull" cx="250" cy="250" r="18"/>';
  html += [235,210,142,116,38,18].map(radius=>`<circle class="board-wire" cx="250" cy="250" r="${radius}"/>`).join('');
  svg.innerHTML=html;
}
function ringToZone(ring){
  return ({double:'double',triple:'triple',single_inner:'singleInner',single_outer:'singleOuter',single_bull:'singleBull',double_bull:'doubleBull'})[ring];
}
function renderBoardEvent(event){
  document.querySelectorAll('.seg.hit,.seg.miss-zone,.seg.advice-target').forEach(element=>element.classList.remove('hit','miss-zone','advice-target'));
  const advice = appState.experience?.game?.advice;
  if(advice?.primary && appState.experience?.game?.status === 'running'){
    const target = advice.primary;
    const segment = $(boardSegmentId(ringToZone(target.ring), target.field));
    if(segment) segment.classList.add('advice-target');
  }
  const pulse=$('boardPulse');
  if(pulse) pulse.className='board-pulse';
  if(!event) return;
  if(event.type==='miss'){
    document.querySelectorAll('.seg').forEach(element=>element.classList.add('miss-zone'));
    if(pulse) pulse.className='board-pulse miss show';
  } else if(event.type==='hit'){
    const segment=$(boardSegmentId(ringToZone(event.ring),event.field));
    if(segment) segment.classList.add('hit');
    if(pulse) pulse.className='board-pulse hit show';
  }
}
function eventFromBoardSegment(segment){
  const match=segment.id.match(/^seg-([A-Za-z]+)-(\d+)$/);
  if(!match) return null;
  const zone=match[1], field=Number(match[2]);
  const definitions={
    double:{ring:'double',multiplier:2,prefix:'D'},
    triple:{ring:'triple',multiplier:3,prefix:'T'},
    singleInner:{ring:'single_inner',multiplier:1,prefix:'S'},
    singleOuter:{ring:'single_outer',multiplier:1,prefix:'S'},
    singleBull:{ring:'single_bull',multiplier:1,prefix:'S'},
    doubleBull:{ring:'double_bull',multiplier:2,prefix:'D'},
  };
  const definition=definitions[zone];
  if(!definition) return null;
  const bull=field===25;
  return {
    type:'hit',
    field,
    ring:definition.ring,
    multiplier:definition.multiplier,
    label:bull ? `${definition.prefix}Bull` : `${definition.prefix}${field}`,
    score:field*definition.multiplier,
    seq:Date.now(),
  };
}

function solveLinear(matrix, vector){
  const size=vector.length;
  for(let column=0;column<size;column++){
    let pivot=column;
    for(let row=column+1;row<size;row++) if(Math.abs(matrix[row][column])>Math.abs(matrix[pivot][column])) pivot=row;
    [matrix[column],matrix[pivot]]=[matrix[pivot],matrix[column]];
    [vector[column],vector[pivot]]=[vector[pivot],vector[column]];
    const divisor=matrix[column][column] || 1e-12;
    for(let item=column;item<size;item++) matrix[column][item]/=divisor;
    vector[column]/=divisor;
    for(let row=0;row<size;row++){
      if(row===column) continue;
      const factor=matrix[row][column];
      for(let item=column;item<size;item++) matrix[row][item]-=factor*matrix[column][item];
      vector[row]-=factor*vector[column];
    }
  }
  return vector;
}
function homographyMatrix(corners){
  const source=[[0,0],[1000,0],[1000,1000],[0,1000]];
  const target=corners.map(point=>[point.x*innerWidth,point.y*innerHeight]);
  const matrix=[], vector=[];
  source.forEach(([x,y],index)=>{
    const [u,v]=target[index];
    matrix.push([x,y,1,0,0,0,-u*x,-u*y]); vector.push(u);
    matrix.push([0,0,0,x,y,1,-v*x,-v*y]); vector.push(v);
  });
  const [a,b,c,d,e,f,g,h]=solveLinear(matrix,vector);
  return `matrix3d(${a},${d},0,${g},${b},${e},0,${h},0,0,1,0,${c},${f},0,1)`;
}
function applyCalibration(){
  const plane=$('projectionPlane');
  if(!plane || !appState.experience?.calibration) return;
  plane.style.transform=homographyMatrix(appState.experience.calibration.corners);
}

function ensureAudio(){
  if(!appState.audio){
    const AudioContext=window.AudioContext||window.webkitAudioContext;
    if(AudioContext) appState.audio=new AudioContext();
  }
  if(appState.audio?.state==='suspended') appState.audio.resume();
  $('audioUnlock')?.classList.add('hidden');
  return appState.audio;
}
function tone(frequency,duration=0.12,delay=0,type='sine',gain=0.12){
  const context=ensureAudio();
  if(!context) return;
  const oscillator=context.createOscillator(), volume=context.createGain();
  oscillator.type=type; oscillator.frequency.value=frequency;
  volume.gain.setValueAtTime(0.0001,context.currentTime+delay);
  volume.gain.exponentialRampToValueAtTime(gain,context.currentTime+delay+0.015);
  volume.gain.exponentialRampToValueAtTime(0.0001,context.currentTime+delay+duration);
  oscillator.connect(volume).connect(context.destination);
  oscillator.start(context.currentTime+delay); oscillator.stop(context.currentTime+delay+duration+0.02);
}
function playEventCue(event,experience){
  if(!isProjector()) return;
  const key=`${event.type}:${event.seq ?? event.action ?? Date.now()}`;
  if(key===appState.lastCueKey) return;
  appState.lastCueKey=key;
  if(event.type==='hit'){
    const base=event.multiplier===3?620:event.multiplier===2?520:event.field===25?760:420;
    tone(base,.16,0,'triangle',.16); tone(base*1.5,.2,.08,'sine',.1);
  } else if(event.type==='miss') tone(110,.28,0,'sawtooth',.08);
  else if(event.type==='continue'||event.type==='next_player') tone(330,.14);
  else if(event.type==='hardware_status'&&event.status==='error') tone(95,.4,0,'sawtooth',.06);
  if(experience.game.status==='finished'){
    [392,523,659,784].forEach((frequency,index)=>tone(frequency,.35,index*.12,'triangle',.13));
  }
}
function startCountdown(){
  clearInterval(appState.countdownTimer);
  let value=3;
  const tick=()=>{
    const element=$('projectorCountdown');
    if(element) element.textContent=value>0?value:'GO';
    if(isProjector()) tone(value>0?330+(3-value)*110:660,.16,0,'triangle',.12);
    if(value<0){
      clearInterval(appState.countdownTimer);
      if(!isProjector() && appState.experience?.screen==='countdown') action('/api/game/live');
    }
    value--;
  };
  tick();
  appState.countdownTimer=setInterval(tick,900);
}

document.addEventListener('click',async event=>{
  const segment=event.target.closest('.seg');
  if(segment){
    const hitEvent=eventFromBoardSegment(segment);
    if(hitEvent && isProjector() && testModeEnabled()){
      await action('/api/event',hitEvent);
      return;
    }
    if(hitEvent && segment.closest('.correction-board') && appState.selectedCorrectionIndex!==null){
      const turnIndex=appState.selectedCorrectionIndex;
      appState.selectedCorrectionIndex=null;
      await action('/api/throw/correct',{turn_index:turnIndex,event:hitEvent});
      return;
    }
  }
  const target=event.target.closest('[data-action]');
  if(!target) return;
  const name=target.dataset.action;
  if(name==='audio'){ ensureAudio(); return; }
  if(name==='choose-players'){ await action('/api/navigation/players'); return; }
  if(name==='home'){ appState.selectedPlayers.clear(); await action('/api/session/close'); return; }
  if(name==='calibrate'){ await action('/api/navigation',{screen:'calibration'}); return; }
  if(name==='toggle-player'){
    appState.selectedPlayers.has(target.dataset.id) ? appState.selectedPlayers.delete(target.dataset.id) : appState.selectedPlayers.add(target.dataset.id);
    renderControl(); return;
  }
  if(name==='start-session'){ await action('/api/session/start',{player_ids:[...appState.selectedPlayers]}); return; }
  if(name==='select-mode'){ await action('/api/game/prepare',{game_type:target.dataset.mode,options:{}}); return; }
  if(name==='set-option'){
    const value=/^-?\d+(\.\d+)?$/.test(target.dataset.value)?Number(target.dataset.value):target.dataset.value;
    const options={...appState.experience.selected_options,[target.dataset.key]:value};
    await action('/api/game/prepare',{game_type:appState.experience.selected_mode,options}); return;
  }
  if(name==='back-games'){ await action('/api/game/next'); return; }
  if(name==='start-game'){ ensureAudio(); await action('/api/game/start'); return; }
  if(name==='continue'){ await action('/api/continue'); return; }
  if(name==='next-player'){ await action('/api/next-player'); return; }
  if(name==='undo'){ await action('/api/undo'); return; }
  if(name==='select-correction'){
    appState.selectedCorrectionIndex=Number(target.dataset.index);
    renderControl(); return;
  }
  if(name==='cancel-correction'){
    appState.selectedCorrectionIndex=null;
    renderControl(); return;
  }
  if(name==='correct-miss'){
    const turnIndex=appState.selectedCorrectionIndex;
    if(turnIndex===null) return;
    appState.selectedCorrectionIndex=null;
    await action('/api/throw/correct',{turn_index:turnIndex,event:{type:'miss',label:'MISS',score:0,seq:Date.now()}});
    return;
  }
  if(name==='test-miss'){
    await action('/api/event',{type:'miss',label:'MISS',score:0,seq:Date.now()});
    return;
  }
  if(name==='next-game'){ await action('/api/game/next'); return; }
  if(name==='end-session'){ await action('/api/session/end'); return; }
  if(name==='close-session'){ appState.selectedPlayers.clear(); await action('/api/session/close'); return; }
  if(name==='save-calibration'){
    const corners=appState.experience.calibration.corners;
    await action('/api/calibration',{...appState.experience.calibration,corners});
    await action('/api/session/close'); return;
  }
});
document.addEventListener('submit',async event=>{
  if(event.target.id!=='newPlayerForm') return;
  event.preventDefault();
  const data=new FormData(event.target);
  const player=await action('/api/players',{name:data.get('name'),avatar:data.get('avatar'),color:data.get('color')});
  appState.selectedPlayers.add(player.id);
  renderControl();
});
document.addEventListener('input',event=>{
  if(!event.target.matches('[data-corner]')) return;
  const corner=Number(event.target.dataset.corner), axis=event.target.dataset.axis;
  appState.experience.calibration.corners[corner][axis]=Number(event.target.value);
  event.target.nextElementSibling.textContent=`${Math.round(Number(event.target.value)*100)}%`;
});
document.addEventListener('change',event=>{
  if(!event.target.matches('[data-corner]')) return;
  action('/api/calibration',appState.experience.calibration);
});
window.addEventListener('resize',()=>{ if(isProjector()) applyCalibration(); });
window.addEventListener('load',()=>{ loadBootstrap(); connectWs(); });
