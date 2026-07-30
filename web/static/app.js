const appState = {
  experience: null,
  wsOk: false,
  serverInstance: null,
  selectedPlayers: new Set(),
  audio: null,
  countdownTimer: null,
  lastCueKey: null,
  projectedEvent: null,
  boardResetTimer: null,
  memoryRevealTimer: null,
  memoryRevealKey: '',
  memoryHidden: false,
  rematchTimer: null,
  rematchArmedUntil: 0,
  selectedCorrectionIndex: null,
  abortArmed: false,
  skipArmed: false,
  geometryTimer: null,
  reportedGeometry: '',
  reportedAudioStatus: '',
  scoreCountdown: null,
  scoreCountdownFrame: null,
  selectedLanguage: 'de',
  languageTouched: false,
  localView: null,
  history: {
    sessions: [], players: [], modes: [], heatmap: null,
    session: null, game: null, replay: null, replayIndex: 0,
    playerId: '', recommendations: null, includeTest: false, loading: false,
  },
};

const BOARD_ORDER = [20,1,18,4,13,6,10,15,2,17,3,19,7,16,8,11,14,9,12,5];
const BOARD_EVENT_VISIBLE_MS = 4000;
const MAX_REMATCH_CONFIRM_MS = 5000;
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
const NEON_MODE_ASSETS = new Set([
  'avoid_bomb','block_drop','boss_fight','candy_cannon','color_clash',
  'cookie_monster','countup','cricket','dart_sweeper','darts_bingo',
  'dragon_eggs','eight_ball','ghost_chase','heart_chase','king_of_board',
  'lightning_round','mini_golf','risk_it','robin_hood','simon_says',
  'space_defender','target_rush','treasure_hunt','x01',
]);
const MODE_AMBIENCE = {
  avoid_bomb:'mines', block_drop:'blocks', boss_fight:'embers',
  candy_cannon:'candy', color_clash:'confetti', cookie_monster:'cookies',
  countup:'streaks', cricket:'leaves', dart_sweeper:'mines',
  darts_bingo:'sparkles', dragon_eggs:'eggs', eight_ball:'billiards',
  ghost_chase:'wisps', heart_chase:'hearts', king_of_board:'sparkles',
  lightning_round:'lightning', mini_golf:'golf', risk_it:'coins',
  robin_hood:'leaves', simon_says:'signals', space_defender:'space',
  target_rush:'streaks', treasure_hunt:'gems', x01:'streaks',
};
const EFFECT_ASSET_NAMES = {
  hearts:'heart', eggs:'egg', cookies:'cookie', candy:'candy',
  blocks:'block', billiards:'billiard', golf:'golf', wisps:'wisp',
  leaves:'leaf', mines:'mine', coins:'coin', gems:'gem',
};
const NEON_EFFECT_ASSETS = new Set([
  'heart','egg','cookie','cookie_moldy','milk','candy','block','billiard',
  'golf','wisp','leaf','mine','mine_explosion','coin','gem','candy_overheat',
  'dragon_scale','dragon_fire',
]);
const OVERLAY_ICON_ASSETS = {
  egg:'egg',
  dragon_scale:'dragon_scale',
  cookie:'cookie',
  cookie_moldy:'cookie_moldy',
  milk:'milk',
  mine:'mine',
};

function $(id){ return document.getElementById(id); }
function isProjector(){ return location.pathname.includes('projector'); }
function language(){
  return appState.experience?.session?.language
    || appState.selectedLanguage
    || appState.experience?.language
    || 'de';
}
function t(key, vars){
  return window.SDB_I18N?.t(key,language(),vars) || key;
}
function localText(value){
  return window.SDB_I18N?.text(value,language()) ?? value;
}
function localMode(mode){
  return window.SDB_I18N?.mode(mode,language()) || mode;
}
function escapeHtml(value){
  return String(value ?? '').replace(/[&<>"']/g, char => ({
    '&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#039;'
  })[char]);
}
function promptMarkup(value){
  const parts=String(value ?? '')
    .split(/\s+·\s+|\s+[–—]\s+|\n+/)
    .map(part=>part.trim())
    .filter(Boolean);
  if(parts.length<2) return escapeHtml(parts[0] || '');
  return `<i class="stacked-prompt">${parts.map(part=>`<em>${escapeHtml(part)}</em>`).join('\n')}</i>`;
}
function overlayIconAsset(icon){
  const name=OVERLAY_ICON_ASSETS[String(icon || '')];
  return name ? effectAsset(name) : '';
}
function visualLegendMarkup(items, placement){
  const entries=(items || []).map(item=>{
    const asset=overlayIconAsset(item.icon);
    if(!asset) return '';
    return `<div class="visual-legend-item variant-${escapeHtml(item.icon)}" style="--legend-color:${escapeHtml(item.color || '#e9a23b')}">
      <img src="${asset}" alt="">
      <p><strong>${escapeHtml(localText(item.label || ''))}</strong><small>${escapeHtml(localText(item.value || ''))}</small></p>
    </div>`;
  }).join('');
  return entries ? `<div class="overlay-visual-legend ${escapeHtml(placement || '')}">${entries}</div>` : '';
}
function modeBySlug(slug){
  return localMode(appState.experience?.modes.find(mode => mode.slug === slug));
}
function modeAsset(slug){
  const safe = /^[a-z0-9_]+$/.test(String(slug || '')) ? slug : 'countup';
  if(appState.experience?.art_theme === 'neon' && NEON_MODE_ASSETS.has(safe)){
    return `/static/assets/themes/neon/modes/${encodeURIComponent(safe)}.webp`;
  }
  return `/static/assets/modes/${encodeURIComponent(safe)}.webp`;
}
function effectAsset(name){
  const safe=/^[a-z0-9_]+$/.test(String(name || '')) ? name : '';
  if(!safe) return '';
  if(appState.experience?.art_theme === 'neon' && NEON_EFFECT_ASSETS.has(safe)){
    return `/static/assets/themes/neon/effects/${encodeURIComponent(safe)}.webp`;
  }
  return `/static/assets/effects/${encodeURIComponent(safe)}.webp`;
}
function avatarEmoji(avatar){
  return AVATARS.find(option => option.id === avatar)?.emoji || '🎯';
}
function currentPlayer(game){
  return game?.players.find(player => player.id === game.current_player_id) || game?.players[0];
}
function isBombEvent(experience,event){
  if(experience?.game?.game_type!=='avoid_bomb' || event?.type!=='hit') return false;
  return Boolean(experience.game.overlay?.danger?.some(item=>
    Number(item.field)===Number(event.field) && String(item.ring)===String(event.ring)
  ));
}
function isMineExplosionEvent(experience,event){
  return experience?.game?.game_type==='dart_sweeper'
    && event?.type==='hit'
    && event?.effect==='mine_explosion';
}
function isExplosionEvent(experience,event){
  return isBombEvent(experience,event) || isMineExplosionEvent(experience,event);
}
function isCandyOverheatEvent(experience,event){
  return experience?.game?.game_type==='candy_cannon'
    && event?.type==='hit'
    && event?.effect==='candy_overheat';
}
function isCandyFireEvent(experience,event){
  return experience?.game?.game_type==='candy_cannon'
    && event?.type==='hit'
    && event?.effect==='candy_fire'
    && Boolean(event?.target_player_id);
}
function isDragonEvent(experience,event,effect){
  return experience?.game?.game_type==='dragon_eggs'
    && event?.type==='hit'
    && event?.effect===effect;
}
function isCookieEvent(experience,event,effect){
  return experience?.game?.game_type==='cookie_monster'
    && event?.type==='hit'
    && event?.effect===effect;
}
function clearScoreCountdown(){
  cancelAnimationFrame(appState.scoreCountdownFrame);
  appState.scoreCountdownFrame=null;
  appState.scoreCountdown=null;
}
function startScoreCountdown(){
  cancelAnimationFrame(appState.scoreCountdownFrame);
  const tick=now=>{
    const countdown=appState.scoreCountdown;
    if(!countdown) return;
    const progress=Math.min(1,Math.max(0,(now-countdown.startedAt)/countdown.duration));
    const eased=1-Math.pow(1-progress,3);
    const value=Math.round(countdown.from+(countdown.to-countdown.from)*eased);
    document.querySelectorAll('[data-score-player]').forEach(element=>{
      if(element.dataset.scorePlayer!==countdown.playerId) return;
      element.textContent=value;
      element.classList.toggle('score-counting',progress<1);
    });
    if(progress<1){
      appState.scoreCountdownFrame=requestAnimationFrame(tick);
    }else{
      appState.scoreCountdownFrame=null;
      appState.scoreCountdown=null;
    }
  };
  tick(performance.now());
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
async function getJson(path){
  const response=await fetch(path);
  const payload=await response.json();
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
function scheduleRematchExpiryCheck(){
  const checkExpiry=()=>{
    if(Date.now()<appState.rematchArmedUntil){
      appState.rematchTimer=setTimeout(checkExpiry,100);
      return;
    }
    appState.rematchArmedUntil=0;
    render();
  };
  appState.rematchTimer=setTimeout(checkExpiry,100);
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
    if(
      payload.dev_reload
      && appState.serverInstance
      && payload.server_instance
      && payload.server_instance !== appState.serverInstance
    ){
      location.reload();
      return;
    }
    if(payload.server_instance) appState.serverInstance=payload.server_instance;
    if(payload.experience) updateExperience(payload.experience, payload.event);
  };
}
function updateExperience(experience, event){
  const previous = appState.experience;
  const rematchWasArmed=Date.now()<appState.rematchArmedUntil;
  const rematchDelay=Math.min(
    MAX_REMATCH_CONFIRM_MS,
    Math.max(0,Number(experience.rematch?.expires_in_ms)||0),
  );
  appState.experience = experience;
  if(experience.session?.language || !appState.languageTouched){
    appState.selectedLanguage=experience.session?.language || experience.language || 'de';
  }
  document.documentElement.lang=language();
  clearTimeout(appState.rematchTimer);
  if(experience.rematch?.armed){
    appState.rematchArmedUntil=Date.now()+rematchDelay;
    scheduleRematchExpiryCheck();
    if(!rematchWasArmed) tone(440,.12,0,'triangle',.08);
  }else{
    appState.rematchArmedUntil=0;
  }
  if(isProjector() && experience.sound?.enabled){
    ensureAudio(true);
  }
  const previousPlayer=previous?.game?.current_player_id;
  const currentPlayerId=experience.game?.current_player_id;
  if(previousPlayer && currentPlayerId && previousPlayer!==currentPlayerId){
    clearTimeout(appState.boardResetTimer);
    appState.projectedEvent=null;
    appState.selectedCorrectionIndex=null;
    appState.skipArmed=false;
  }
  const memoryKey=experience.game?.game_type==='simon_says' && experience.game?.status==='running'
    ? `${experience.game.round_number}:${experience.game.current_player_id}`
    : '';
  if(memoryKey && memoryKey!==appState.memoryRevealKey){
    clearTimeout(appState.memoryRevealTimer);
    appState.memoryRevealKey=memoryKey;
    appState.memoryHidden=false;
    appState.memoryRevealTimer=setTimeout(()=>{
      appState.memoryHidden=true;
      if(isProjector() && appState.experience?.screen==='playing') renderProjector();
    },3000);
  }else if(!memoryKey){
    clearTimeout(appState.memoryRevealTimer);
    appState.memoryRevealKey='';
    appState.memoryHidden=false;
  }
  if(previous?.screen === 'game_result' && experience.screen !== 'game_result'){
    clearTimeout(appState.boardResetTimer);
    appState.projectedEvent=null;
  }
  if(event){
    const isThrow=event.type==='hit' || event.type==='miss';
    const lastThrow=experience.game?.throws?.at(-1);
    const throwWasCounted=!isThrow || (lastThrow && Number(lastThrow.seq)===Number(event.seq));
    if(throwWasCounted) playEventCue(event, experience);
    if(isThrow && throwWasCounted){
      if(isProjector() && (isBombEvent(experience,event) || isCandyFireEvent(experience,event))){
        const candyFire=isCandyFireEvent(experience,event);
        const playerId=candyFire
          ? event.target_player_id
          : lastThrow?.player_id || experience.game?.current_player_id;
        const from=previous?.game?.players?.find(player=>player.id===playerId)?.score;
        const to=experience.game?.players?.find(player=>player.id===playerId)?.score;
        if(Number.isFinite(from) && Number.isFinite(to) && from!==to){
          appState.scoreCountdown={
            playerId,
            from:Number(from),
            to:Number(to),
            startedAt:performance.now()+(candyFire?420:0),
            duration:candyFire?850:1400,
          };
        }
      }else if(isProjector()){
        clearScoreCountdown();
      }
      appState.selectedCorrectionIndex=null;
      appState.abortArmed=false;
      appState.skipArmed=false;
      appState.projectedEvent=event;
      clearTimeout(appState.boardResetTimer);
      appState.boardResetTimer=setTimeout(()=>{
        if(appState.experience?.screen === 'game_result') return;
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
  if(isProjector() && appState.scoreCountdown) startScoreCountdown();
}
function renderConnection(){
  const element = $('wsStatus');
  if(!element) return;
  const subtitle=$('brandSubtitle');
  if(subtitle) subtitle.textContent=t('session_control');
  const brand=document.querySelector('.brand-button');
  if(brand) brand.setAttribute('aria-label',t('home'));
  const hardware=appState.experience?.hardware;
  const boardReady=!hardware?.enabled || hardware.status==='connected';
  const cssClass=!appState.wsOk?'':boardReady?'online':'searching';
  const label=!appState.wsOk?'OFFLINE':boardReady?'LIVE':hardware?.status==='error'?t('board_error'):t('board_searching');
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
  if(appState.localView==='history'){
    root.innerHTML=controlHistory();
    if($('historyHeatmapBoard')){
      buildBoardElement($('historyHeatmapBoard'));
      applyHistoryHeatmap();
    }
    if($('replayBoard')){
      buildBoardElement($('replayBoard'));
      applyReplayFrame();
    }
    return;
  }
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
      <div class="kicker">${t('ready_when_you_are')}</div>
      <h1>${escapeHtml(t('hero_title'))}</h1>
      <p>${t('hero_copy')}</p>
      <div class="button-row">
        ${actionButton(t('start_session'), 'choose-players')}
        ${actionButton(t('calibrate'), 'calibrate', 'ghost')}
        ${actionButton(t('statistics'), 'open-history', 'ghost')}
      </div>
    </div>
    <div class="hero-orbit" aria-hidden="true"><i></i><i></i><i></i><b>◎</b></div>
  </section>`;
}

function playerCard(player, selected){
  return `<button class="player-select ${selected ? 'selected' : ''}" data-action="toggle-player" data-id="${escapeHtml(player.id)}">
    <span class="avatar avatar-${escapeHtml(player.avatar)}" style="--player:${escapeHtml(player.color)}" aria-label="${escapeHtml(player.avatar)}">${avatarEmoji(player.avatar)}</span>
    <span><b>${escapeHtml(player.name)}</b><small>${selected ? t('selected') : t('tap')}</small></span>
    <i>${selected ? '✓' : '+'}</i>
  </button>`;
}
function controlPlayers(){
  const players = appState.experience.players;
  const cards = players.map(player => playerCard(player, appState.selectedPlayers.has(player.id))).join('');
  return `<section class="control-scene">
    ${sceneHeader(t('step_players'),t('who_plays'),t('choose_players'))}
    <section class="language-picker" aria-label="${t('session_language')}">
      <span>${t('session_language')}</span>
      <div class="segmented">
        <button class="${appState.selectedLanguage==='de'?'selected':''}" data-action="set-language" data-language="de">DE · ${t('german')}</button>
        <button class="${appState.selectedLanguage==='en'?'selected':''}" data-action="set-language" data-language="en">EN · ${t('english')}</button>
      </div>
    </section>
    <div class="player-layout">
      <div class="selection-grid">${cards || `<div class="empty-state">${t('create_first_player')}</div>`}</div>
      <form id="newPlayerForm" class="new-player-card">
        <h2>${t('new_player')}</h2>
        <label>${t('name')}<input name="name" maxlength="32" autocomplete="off" placeholder="${t('player_name')}" required></label>
        <div class="choice-label">${t('avatar')}</div>
        <div class="avatar-choices">${AVATARS.map((avatar,index) =>
          `<label class="mini-choice" title="${escapeHtml(localText(avatar.label))}"><input type="radio" name="avatar" value="${escapeHtml(avatar.id)}" ${index===0?'checked':''}><span>${avatar.emoji}</span></label>`
        ).join('')}</div>
        <div class="choice-label">${t('color')}</div>
        <div class="color-choices">${COLORS.map((color,index) =>
          `<label><input type="radio" name="color" value="${color}" ${index===0?'checked':''}><span style="--swatch:${color}"></span></label>`
        ).join('')}</div>
        ${actionButton(t('create_player'),'create-player','secondary','type="submit"')}
      </form>
    </div>
    <footer class="sticky-actions">
      ${actionButton(t('back'),'home','ghost')}
      <div>${t('players_selected',{count:`<b>${appState.selectedPlayers.size}</b>`})}</div>
      ${actionButton(t('continue_games'),'start-session','primary',appState.selectedPlayers.size ? '' : 'disabled')}
    </footer>
  </section>`;
}

function modeCard(mode){
  mode=localMode(mode);
  return `<button class="mode-card" data-action="select-mode" data-mode="${escapeHtml(mode.slug)}" style="--accent:${escapeHtml(mode.accent)}">
    <img src="${modeAsset(mode.slug)}" alt="" loading="eager" onerror="this.onerror=null;this.src='/static/assets/modes/countup.webp'">
    <span class="mode-shade"></span>
    <span class="mode-copy"><small>${escapeHtml(mode.tagline)}</small><b>${escapeHtml(mode.title)}</b><em>${escapeHtml(mode.description)}</em></span>
    <i>→</i>
  </button>`;
}
function controlGameSelect(){
  const session = appState.experience.session;
  return `<section class="control-scene">
    ${sceneHeader(t('step_games'),t('choose_game'),t('modes_ready',{count:session?.players.length || 0}))}
    ${sessionScoreStrip()}
    <div class="mode-grid">${appState.experience.modes.map(modeCard).join('')}</div>
    <footer class="sticky-actions">
      <div class="footer-actions">${actionButton(t('statistics'),'open-history','ghost')}${actionButton(t('end_session'),'end-session','ghost')}</div>
      <div class="session-people">${(session?.players || []).map(player => `<span style="--player:${escapeHtml(player.color)}">${escapeHtml(player.name)}</span>`).join('')}</div>
    </footer>
  </section>`;
}

function optionControl(option, selected){
  return `<div class="option-block"><div class="choice-label">${escapeHtml(localText(option.label))}</div><div class="segmented">
    ${option.choices.map(choice => `<button class="${String(choice.value)===String(selected) ? 'selected' : ''}" data-action="set-option" data-key="${escapeHtml(option.key)}" data-value="${escapeHtml(choice.value)}">${escapeHtml(localText(choice.label))}</button>`).join('')}
  </div></div>`;
}
function instructionSteps(mode){
  return mode.instructions.map((step,index) => `<article class="instruction-step">
    <span>${index+1}</span><div><h3>${escapeHtml(step.title)}</h3><p>${escapeHtml(step.body)}</p></div>
  </article>`).join('');
}
function controlGraphic(icon){
  const paths={
    left:'<path d="M20 8 6 20l14 12M7 20h29"/>',
    right:'<path d="m20 8 14 12-14 12M5 20h29"/>',
    rotate_left:'<path d="M11 13H4V6"/><path d="M5 13a16 16 0 1 1 2 17"/>',
    rotate_right:'<path d="M29 13h7V6"/><path d="M35 13a16 16 0 1 0-2 17"/>',
    drop:'<path d="M20 4v23M10 18l10 10 10-10"/><path d="M6 35h28"/>',
  };
  return `<svg viewBox="0 0 40 40" aria-hidden="true" focusable="false">${paths[icon] || paths.drop}</svg>`;
}
function controlLegend(items,placement){
  if(!items?.length) return '';
  return `<aside class="control-legend ${escapeHtml(placement)}">
    <span>${t('controls')}</span>
    <div>${items.map(item=>`<div class="control-legend-row">
      <i class="control-legend-icon">${controlGraphic(item.icon)}</i>
      <i class="control-legend-color" style="--control-color:${escapeHtml(item.color || '#28e7ff')};--control-color-2:${escapeHtml(item.secondary_color || item.color || '#28e7ff')}"></i>
      <b>${escapeHtml(localText(item.label || ''))}${item.detail?`<small>${escapeHtml(localText(item.detail))}</small>`:''}</b>
    </div>`).join('')}</div>
  </aside>`;
}
function controlInstructions(){
  const mode = modeBySlug(appState.experience.selected_mode);
  if(!mode) return controlGameSelect();
  return `<section class="control-scene instruction-control" style="--accent:${escapeHtml(mode.accent)};--hero:url('${modeAsset(mode.slug)}')">
    <div class="instruction-hero">
      <div>${sceneHeader(mode.tagline,mode.title,mode.description)}</div>
    </div>
    <div class="instruction-content">
      <div class="instruction-list">${controlLegend(mode.control_legend,'control-instructions')}${instructionSteps(mode)}</div>
      <div class="game-options">${mode.options.map(option => optionControl(option,appState.experience.selected_options[option.key])).join('')}</div>
    </div>
    <footer class="sticky-actions">
      ${actionButton(t('other_game'),'back-games','ghost')}
      ${actionButton(t('start_game'),'start-game')}
    </footer>
  </section>`;
}

function controlCountdown(){
  return `<section class="control-scene centered-scene">
    <div class="radar-loader"><i></i><b id="countdownValue" aria-live="polite">3</b></div>
    <div class="kicker">${t('projector_running')}</div>
    <h1>${t('game_starts')}</h1>
    <p>${t('line_ready')}</p>
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
    <div><b>${escapeHtml(player.name)}</b>${game.game_type==='cricket' ? marks(player,game.cricket_targets) : ''}${game.game_type==='darts_bingo' ? bingoCard(player.marks) : ''}</div>
    <strong>${player.score}</strong>
  </article>`).join('');
}
function bingoCard(playerMarks){
  if(!playerMarks) return '';
  return `<div class="bingo-card">${Object.entries(playerMarks).sort(([a],[b])=>Number(a)-Number(b)).map(([,cell])=>
    `<span class="${cell.done?'done':''}">${escapeHtml(cell.label)}</span>`
  ).join('')}</div>`;
}
function currentTurnThrows(game){
  return game.darts_in_turn > 0 ? game.throws.slice(-game.darts_in_turn) : [];
}
function turnDartCards(game){
  const throws=currentTurnThrows(game);
  return [0,1,2].map(index=>{
    const dart=throws[index];
    if(!dart){
      return `<div class="turn-dart empty"><span>${index+1}</span><b>—</b><small>${language()==='en'?'OPEN':'NOCH OFFEN'}</small></div>`;
    }
    return `<button class="turn-dart ${appState.selectedCorrectionIndex===index?'selected':''}" data-action="select-correction" data-index="${index}">
      <span>${index+1}</span><b>${escapeHtml(dart.label || 'MISS')}</b><small>${dart.score} ${language()==='en'?'POINTS · TAP TO CORRECT':'PUNKTE · ANTIPPEN ZUM KORRIGIEREN'}</small>
    </button>`;
  }).join('');
}
function correctionPanel(){
  const selected=appState.selectedCorrectionIndex;
  if(selected===null){
    return `<div class="correction-hint">${t('correction_hint')}</div>`;
  }
  return `<section class="correction-panel">
    <div class="correction-board-shell">
      <svg id="dartboardSvg" class="dartboard-svg correction-board" viewBox="0 0 500 500" aria-label="Korrektur-Dartboard"></svg>
    </div>
    <div class="correction-copy">
      <div class="kicker">${t('correct_throw',{number:selected+1})}</div>
      <h2>${t('tap_actual_segment')}</h2>
      <p>${t('correction_copy')}</p>
      <div class="correction-actions">
        ${actionButton(t('count_as_miss'),'correct-miss','danger')}
        ${actionButton(t('cancel'),'cancel-correction','ghost')}
      </div>
    </div>
  </section>`;
}
function x01AdvicePanel(game){
  const advice = game.advice;
  if(game.game_type !== 'x01' || !advice || !advice.primary) return '';
  const sequence = (advice.sequence || []).map(dart => `<b>${escapeHtml(dart.label)}</b>`).join('<span>→</span>');
  const follow = advice.setup?.remaining_checkout?.length ? advice.setup.remaining_checkout.map(dart => dart.label).join(' → ') : '';
  const title = advice.status === 'checkout'
    ? (language()==='en'?'Checkout available':'Finish möglich')
    : advice.status === 'setup'
      ? (language()==='en'?'Set up':'Clever stellen')
      : (language()==='en'?'Score down':'Runterspielen');
  return `<aside class="x01-advice ${escapeHtml(advice.status)}">
    <div><span class="kicker">X01 ADVISOR · ${advice.darts_left} ${t('darts')}</span><h2>${title}: ${escapeHtml(advice.primary.label)}</h2><p>${escapeHtml(localText(advice.message || ''))}</p></div>
    ${sequence ? `<div class="checkout-sequence">${sequence}</div>` : ''}
    ${follow ? `<small>${language()==='en'?'Then':'Danach'}: ${escapeHtml(follow)}</small>` : ''}
  </aside>`;
}

function overlayActionButtons(game){
  const actions = game.overlay?.actions || [];
  if(!actions.length) return '';
  return actions.map(item => `<button class="action-button primary" data-action="game-action" data-game-action="${escapeHtml(item.id)}" ${item.enabled===false?'disabled':''}>${escapeHtml(item.label || item.id)}</button>`).join('');
}
function controlModePrompt(game){
  const overlay=game.overlay;
  if(!overlay?.prompt || game.game_type==='x01') return '';
  const legend=modeBySlug(game.game_type)?.control_legend;
  if(legend?.length) return controlLegend(legend,'control-live');
  const detail=overlay.combo?.count
    ? `COMBO ×${overlay.combo.count}`
    : Number.isFinite(overlay.pot)
      ? `POT ${overlay.pot}`
      : '';
  const visualLegend=visualLegendMarkup(overlay.visual_legend,'control-visual-legend');
  return `<aside class="control-mode-prompt">
    <span>${t('current_task')}</span>
    <b>${promptMarkup(localText(overlay.prompt))}</b>
    ${detail ? `<small>${escapeHtml(detail)}</small>` : ''}
    ${visualLegend}
  </aside>`;
}
function genericPanel(panel, compact=false){
  if(!panel) return '';
  if(panel.kind==='dragon_heat') return dragonHeatPanel(panel,compact);
  const progress=panel.progress && Number.isFinite(Number(panel.progress.max))
    ? `<i class="generic-progress" style="--progress:${Math.max(0,Math.min(100,Number(panel.progress.value||0)/Math.max(1,Number(panel.progress.max))*100))}%"></i>`
    : '';
  const stats=(panel.stats||[]).map(item=>`<b><small>${escapeHtml(localText(item.label||''))}</small><strong>${escapeHtml(localText(item.value??''))}</strong></b>`).join('');
  const rows=(panel.rows||[]).map(item=>`<div class="${item.state?`state-${escapeHtml(item.state)}`:''}"><span>${escapeHtml(localText(item.label||''))}</span><strong>${escapeHtml(localText(item.value??''))}</strong></div>`).join('');
  const grid=panel.grid ? `<div class="generic-grid" style="--columns:${Math.max(1,Math.min(12,Number(panel.grid.columns)||1))}">${(panel.grid.cells||[]).map(cell=>`<i class="${cell.state?`state-${escapeHtml(cell.state)}`:''}" title="${escapeHtml(cell.label||'')}">${escapeHtml(cell.value??'')}</i>`).join('')}</div>` : '';
  return `<section class="generic-mode-panel ${compact?'compact':''}">
    <span>${escapeHtml(localText(panel.title||'SPIELSTATUS'))}</span>
    ${panel.headline?`<h3>${escapeHtml(localText(panel.headline))}</h3>`:''}
    ${panel.subline?`<p>${escapeHtml(localText(panel.subline))}</p>`:''}
    ${progress}
    ${stats?`<div class="generic-stats">${stats}</div>`:''}
    ${rows?`<div class="generic-rows">${rows}</div>`:''}
    ${grid}
  </section>`;
}
function dragonHeatPanel(panel,compact=false){
  const heat=Math.max(0,Math.min(2,Number(panel.heat)||0));
  const reaction=isDragonEvent(appState.experience,appState.projectedEvent,'dragon_fire')
    ? 'fire'
    : isDragonEvent(appState.experience,appState.projectedEvent,'dragon_scale')
      ? 'scale'
      : isDragonEvent(appState.experience,appState.projectedEvent,'dragon_egg')
        ? 'egg'
        : '';
  return `<section class="dragon-heat-panel ${compact?'compact':''} heat-${heat} react-${reaction}">
    <div class="dragon-heat-copy">
      <span>${escapeHtml(localText(panel.title||'DRACHEN-HITZE'))}</span>
      <h3>${escapeHtml(localText(panel.headline||`${heat}/3 FLAMMEN`))}</h3>
      <p>${escapeHtml(localText(panel.subline||'Die dritte Schuppe entfacht das Feuer'))}</p>
    </div>
    <img src="${effectAsset(heat>=2?'dragon_fire':'dragon_scale')}" alt="">
    <div class="dragon-heat-pips">${[0,1,2].map(index=>`<i class="${index<heat?'lit':''}"></i>`).join('')}</div>
  </section>`;
}
function controlPlaying(){
  const game = appState.experience.game;
  const mode = modeBySlug(game.game_type);
  return `<section class="control-scene play-control" style="--accent:${escapeHtml(mode?.accent || '#28e7ff')}">
    <div class="play-heading">
    <div><div class="kicker">${escapeHtml(mode?.title || game.game_type)} · ${t('round')} ${game.round_number}</div><h1>${game.status==='hold'?t('turn_finished'):t('player_turn',{name:escapeHtml(currentPlayer(game)?.name || '')})}</h1></div>
      <div class="turn-counter"><span>${game.darts_in_turn}</span><small>/ 3 ${t('darts')}</small><b>${game.turn_score} ${t('points_short')}</b></div>
    </div>
    ${x01AdvicePanel(game)}
    ${controlModePrompt(game)}
    <div class="turn-darts">${turnDartCards(game)}</div>
    ${correctionPanel()}
    <div class="scoreboard">${scoreboard(game)}</div>
    ${genericPanel(game.overlay?.panel)}
    <div class="operator-panel">
      ${overlayActionButtons(game)}
      ${game.status==='hold' ? actionButton(t('continue_player'),'continue','primary') : skipPlayerControls()}
      ${actionButton(t('undo'),'undo','danger')}
      ${abortControls()}
    </div>
  </section>`;
}
function skipPlayerControls(){
  if(!appState.skipArmed){
    return actionButton(t('early_end'),'arm-skip','secondary');
  }
  return `<div class="skip-confirm">
    <b>${t('skip_confirm')}</b>
    ${actionButton(t('switch_player'),'next-player','danger')}
    ${actionButton(t('keep_playing'),'cancel-skip','ghost')}
  </div>`;
}
function abortControls(){
  if(!appState.abortArmed){
    return actionButton(t('abort_game'),'arm-abort','ghost');
  }
  return `<div class="abort-confirm">
    <b>${t('abort_warning')}</b>
    ${actionButton(t('confirm_abort'),'abort-game','danger')}
    ${actionButton(t('keep_playing'),'cancel-abort','ghost')}
  </div>`;
}

function sessionStandings(){
  return appState.experience.session_statistics || [];
}
function sessionScoreStrip(){
  const standings=sessionStandings();
  if(!standings.length) return '';
  return `<section class="session-score-strip">
    <span>${t('session_score')}</span>
    <div>${standings.map((player,index)=>`<b style="--player:${escapeHtml(player.color)}"><i>${index+1}</i>${escapeHtml(player.name)}<strong>${player.session_points}</strong></b>`).join('')}</div>
  </section>`;
}

function winner(){
  const game = appState.experience.game;
  return game.players.find(player => player.id === game.winner_id);
}
function resultCopy(game, champion){
  if(game.result_type==='team_win'){
    return {icon:'★', label:t('team_win'), title:t('together_done'), points:t('for_everyone')};
  }
  if(champion || game.result_type==='individual_win'){
    return {icon:'♛', label:t('game_decided'), title:champion?.name || t('game_won'), points:t('session_points_award')};
  }
  const draw=game.result_type==='draw' || String(game.message || '').startsWith('Unentschieden');
  return draw
    ? {icon:'=', label:t('draw'), title:t('draw_title'), points:t('no_session_points')}
    : {icon:'☠', label:t('challenge_lost'), title:t('boss_wins'), points:t('no_session_points')};
}
function rematchPrompt(compact=false){
  const armed=Date.now()<appState.rematchArmedUntil;
  const title=armed ? t('press_again') : t('double_press');
  const detail=armed ? t('rematch_confirm') : t('same_game_rotates');
  return compact
    ? `<small class="rematch-prompt ${armed?'armed':''}">${title} · ${t('rematch')}</small>`
    : `<aside class="rematch-prompt ${armed?'armed':''}"><span>${t('board_button')}</span><b>${title}</b><small>${detail}</small></aside>`;
}
function controlGameResult(){
  const game=appState.experience.game;
  const champion = winner();
  const result=resultCopy(game,champion);
  return `<section class="control-scene result-control">
    <div class="trophy-orbit">${result.icon}</div>
    <div class="kicker">${t('game_complete')}</div>
    <h1>${escapeHtml(result.title)}</h1>
    <p>${game.result_type==='team_win'
      ? escapeHtml(localText(game.message || 'Das Team gewinnt gemeinsam.'))
      : champion
        ? escapeHtml(language()==='en'?'wins the game and earns 3 session points.':'holt sich den Sieg und 3 Sessionpunkte.')
        : escapeHtml(localText(game.message || 'Keine Sessionpunkte in diesem Spiel.'))}</p>
    ${sessionScoreStrip()}
    ${rematchPrompt()}
    <div class="result-actions">
      ${actionButton(t('back_to_games'),'next-game')}
    </div>
  </section>`;
}
function statCards(){
  const stats=sessionStandings().length ? sessionStandings() : appState.experience.statistics;
  return stats.map(stat => `<article class="stat-card" style="--player:${escapeHtml(stat.color)}">
    <h3>${escapeHtml(stat.name)}</h3>
    <div>${Number.isFinite(stat.session_points)?`<span><b>${stat.session_points}</b><small>${t('session_points')}</small></span>`:''}<span><b>${stat.wins}</b><small>${t('wins')}</small></span><span><b>${stat.games}</b><small>${t('games')}</small></span><span><b>${stat.win_rate}%</b><small>${t('win_rate')}</small></span><span><b>${stat.darts}</b><small>${t('darts')}</small></span><span><b>${stat.three_dart_average}</b><small>${t('three_dart_average')}</small></span></div>
  </article>`).join('');
}
function controlSessionSummary(){
  return `<section class="control-scene">
    ${sceneHeader(t('session_complete'),t('highlights'),t('saved'))}
    <div class="stats-grid">${statCards()}</div>
    <footer class="sticky-actions">
      ${actionButton(t('statistics'),'open-history','ghost')}
      ${actionButton(t('home'),'close-session')}
    </footer>
  </section>`;
}

function formatDate(value){
  if(!value) return '—';
  return new Intl.DateTimeFormat(language()==='en'?'en-GB':'de-DE',{
    dateStyle:'medium',timeStyle:'short',
  }).format(new Date(value));
}
async function loadHistory(){
  const h=appState.history;
  h.loading=true;
  renderControl();
  const suffix=h.includeTest?'?include_test=true':'';
  try{
    const [sessions,players,modes]=await Promise.all([
      getJson('/api/history/sessions?limit=100'),
      getJson(`/api/statistics/players${suffix}`),
      getJson(`/api/statistics/modes${suffix}`),
    ]);
    h.sessions=sessions.sessions || [];
    h.players=players.players || [];
    h.modes=modes.modes || [];
    await loadHistoryHeatmap();
  }catch(error){
    showToast(error.message);
  }finally{
    h.loading=false;
    renderControl();
  }
}
async function loadHistoryHeatmap(){
  const h=appState.history;
  const params=new URLSearchParams();
  if(h.playerId) params.set('player_id',h.playerId);
  if(h.includeTest) params.set('include_test','true');
  h.heatmap=await getJson(`/api/statistics/heatmap?${params}`);
}
async function openHistory(){
  appState.localView='history';
  appState.history.session=null;
  appState.history.game=null;
  appState.history.replay=null;
  renderControl();
  await loadHistory();
}
async function openHistorySession(id){
  appState.history.session=await getJson(`/api/history/sessions/${encodeURIComponent(id)}`);
  appState.history.game=null;
  appState.history.replay=null;
  renderControl();
}
async function openHistoryGame(id){
  const [game,replay]=await Promise.all([
    getJson(`/api/history/games/${encodeURIComponent(id)}`),
    getJson(`/api/history/games/${encodeURIComponent(id)}/replay`),
  ]);
  appState.history.game=game;
  appState.history.replay=replay;
  appState.history.replayIndex=Math.max(0,(replay.events?.length || 1)-1);
  renderControl();
}
function historyStatCard(player){
  return `<article class="history-player" style="--player:${escapeHtml(player.color)}">
    <header><span>${avatarEmoji(player.avatar)}</span><div><b>${escapeHtml(player.name)}</b><small>${player.games} ${t('games')}</small></div></header>
    <div><b>${player.wins}<small>${t('wins')}</small></b><b>${player.win_rate}%<small>${t('win_rate')}</small></b><b>${player.three_dart_average}<small>${t('three_dart_average')}</small></b><b>${player.darts}<small>${t('throws')}</small></b></div>
    <button data-action="history-player" data-player="${escapeHtml(player.id)}">${t('heatmap')} · ${t('training')}</button>
  </article>`;
}
function modeOptionSummary(item){
  const mode=modeBySlug(item.game_type);
  return Object.entries(item.options || {}).map(([key,value])=>{
    const option=mode?.options?.find(entry=>entry.key===key);
    const choice=option?.choices?.find(entry=>String(entry.value)===String(value));
    return `${localText(option?.label || key)}: ${localText(choice?.label || value)}`;
  }).join(' · ');
}
function historyOverview(){
  const h=appState.history;
  const sessions=h.sessions.map(session=>`<button class="history-session-row" data-action="history-session" data-id="${escapeHtml(session.id)}">
    <span><b>${formatDate(session.started_at)}</b><small>${session.language.toUpperCase()} · ${session.player_count} ${t('players')}</small></span>
    <span><b>${session.finished_games}</b><small>${t('completed_games')}</small></span><i>→</i>
  </button>`).join('');
  const modes=h.modes.map(item=>{
    const mode=modeBySlug(item.game_type);
    return `<article class="history-mode-row"><span><b>${escapeHtml(mode?.title || item.game_type)}</b><small>v${item.ruleset_version} · ${item.starts} Starts<br>${escapeHtml(modeOptionSummary(item))}</small></span><b>${item.success_rate}%<small>${t('success_rate')}</small></b><b>${item.completion_rate}%<small>${t('completion_rate')}</small></b></article>`;
  }).join('');
  return `<div class="history-dashboard">
    <section><h2>${t('players')}</h2><div class="history-player-grid">${h.players.map(historyStatCard).join('') || `<div class="empty-state">${t('no_data')}</div>`}</div></section>
    <section><h2>${t('heatmap')}</h2>
      <div class="history-heatmap-layout">
        <div class="history-board"><svg id="historyHeatmapBoard" class="dartboard-svg" viewBox="0 0 500 500" aria-label="${t('heatmap')}"></svg></div>
        <div class="heatmap-copy"><strong>${h.heatmap?.board_hits || 0}</strong><span>${t('hits')}</span><p>${h.heatmap?.misses || 0} Miss · ${h.heatmap?.total_darts || 0} ${t('throws')}<br>${h.playerId ? escapeHtml(h.players.find(item=>item.id===h.playerId)?.name || '') : t('production_only')}</p>
          ${h.recommendations ? `<h3>${t('recommendation')}</h3><div class="training-recommendations">${h.recommendations.recommendations.map(item=>`<b>${escapeHtml(item.ring)} ${item.field}<small>${item.attempts ? `${item.success_rate}% · ${item.attempts} ${t('throws')}` : t('insufficient_data')}</small></b>`).join('')}</div>` : ''}
        </div>
      </div>
    </section>
    <section><h2>${t('sessions')}</h2><div class="history-list">${sessions || `<div class="empty-state">${t('no_data')}</div>`}</div></section>
    <section><h2>${t('game_modes')}</h2><div class="history-mode-list">${modes || `<div class="empty-state">${t('no_data')}</div>`}</div></section>
  </div>`;
}
function historySessionDetail(session){
  const games=(session.games || []).map(game=>{
    const mode=modeBySlug(game.game_type);
    return `<button class="history-game-row" data-action="history-game" data-id="${escapeHtml(game.id)}">
      <img src="${modeAsset(game.game_type)}" alt=""><span><b>${escapeHtml(mode?.title || game.game_type)}</b><small>${formatDate(game.started_at)} · ${escapeHtml(game.status)} · ${game.darts} ${t('throws')}</small></span><i>→</i>
    </button>`;
  }).join('');
  return `<section class="history-detail">
    <button class="history-back" data-action="history-overview">← ${t('overview')}</button>
    ${sceneHeader(t('sessions'),formatDate(session.started_at),`${session.players.length} ${t('players')} · ${session.games.length} ${t('games')}`)}
    <div class="history-session-players">${session.statistics.map(historyStatCard).join('')}</div>
    <div class="history-list">${games || `<div class="empty-state">${t('no_data')}</div>`}</div>
  </section>`;
}
function historyReplay(game,replay){
  const events=replay?.events || [];
  const index=Math.min(appState.history.replayIndex,Math.max(0,events.length-1));
  const item=events[index] || {};
  const frame=item.frame || game.final_state || game.initial_state || {};
  const player=frame.players?.find(entry=>entry.id===item.player_id);
  return `<section class="history-detail replay-view">
    <button class="history-back" data-action="history-session-back">← ${t('sessions')}</button>
    ${sceneHeader(t('replay'),modeBySlug(game.game_type)?.title || game.game_type,`${formatDate(game.started_at)} · ${game.environment.toUpperCase()} · Ruleset v${game.ruleset_version}`)}
    <div class="replay-layout">
      <div class="history-board"><svg id="replayBoard" class="dartboard-svg" viewBox="0 0 500 500" aria-label="${t('replay')}"></svg></div>
      <div class="replay-state">
        <span>${t('event')} ${events.length ? index+1 : 0}/${events.length}</span>
        <h2>${escapeHtml(item.payload?.label || item.event_type || '—')}</h2>
        <p>${escapeHtml(player?.name || '')} · ${t('round')} ${frame.round_number || 1}</p>
        <div class="replay-scores">${(frame.players || []).map(entry=>`<b style="--player:${escapeHtml(entry.color)}">${escapeHtml(entry.name)}<strong>${entry.score}</strong></b>`).join('')}</div>
        <input id="replayRange" type="range" min="0" max="${Math.max(0,events.length-1)}" value="${index}" ${events.length?'':'disabled'}>
        <div class="button-row">${actionButton(t('previous'),'replay-prev','ghost',index<=0?'disabled':'')}${actionButton(t('next'),'replay-next','secondary',index>=events.length-1?'disabled':'')}</div>
      </div>
    </div>
    <div class="throw-ledger">${game.throws.map(item=>`<span class="${item.outcome}"><b>${escapeHtml(item.event.label || 'MISS')}</b><small>${escapeHtml(item.outcome)} · ${item.mode_points >= 0 ? '+' : ''}${item.mode_points}</small></span>`).join('')}</div>
  </section>`;
}
function controlHistory(){
  const h=appState.history;
  const body=h.loading
    ? '<div class="history-loading"><i></i></div>'
    : h.game
      ? historyReplay(h.game,h.replay)
      : h.session
        ? historySessionDetail(h.session)
        : historyOverview();
  return `<section class="control-scene history-control">
    <header class="history-toolbar">
      <div><div class="kicker">${t('statistics')}</div><h1>${t('stats_title')}</h1><p>${t('stats_copy')}</p></div>
      <div class="history-toolbar-actions">
        <label class="test-data-toggle"><input type="checkbox" data-action="history-test" ${h.includeTest?'checked':''}><span>${t('test_data')}</span></label>
        <a class="action-button secondary history-export" href="/api/data/export" download>${t('export_data')}</a>
        ${actionButton(t('close'),'close-history','ghost')}
      </div>
    </header>
    ${body}
  </section>`;
}
function applyHistoryHeatmap(){
  const segments=appState.history.heatmap?.segments || [];
  const max=Math.max(1,...segments.map(item=>Number(item.darts)||0));
  for(const item of segments){
    const zone=ringToZone(item.ring);
    const segment=$(boardSegmentId(zone,item.field));
    if(!segment) continue;
    const intensity=.15+(Number(item.darts)||0)/max*.85;
    segment.style.fill=`color-mix(in srgb, var(--cyan) ${Math.round(intensity*100)}%, #101820)`;
    segment.style.opacity=String(.45+intensity*.55);
  }
}
function applyReplayFrame(){
  const events=appState.history.replay?.events || [];
  const item=events[appState.history.replayIndex];
  if(!item?.payload) return;
  const zone=ringToZone(item.payload.ring);
  const segment=$(boardSegmentId(zone,item.payload.field));
  if(segment) segment.classList.add('hit');
}

function cornerControls(calibration){
  const labels = [t('top_left'),t('top_right'),t('bottom_right'),t('bottom_left')];
  return calibration.corners.map((corner,index) => `<fieldset class="corner-control"><legend>${labels[index]}</legend>
    <label>X <input type="range" min="0" max="1" step="0.002" value="${corner.x}" data-corner="${index}" data-axis="x"><output>${Math.round(corner.x*100)}%</output></label>
    <label>Y <input type="range" min="0" max="1" step="0.002" value="${corner.y}" data-corner="${index}" data-axis="y"><output>${Math.round(corner.y*100)}%</output></label>
  </fieldset>`).join('');
}
function controlCalibration(){
  const geometry=appState.experience.projector_geometry || {width:1600,height:900};
  return `<section class="control-scene calibration-control">
    ${sceneHeader(t('one_time_setup'),t('calibrate_title'),t('calibrate_copy'))}
    <div class="calibration-grid">${cornerControls(appState.experience.calibration)}</div>
    ${artThemeSetup()}
    ${soundSetup()}
    <p class="calibration-note">${t('calibration_note',{width:`<b>${geometry.width}`,height:`${geometry.height}</b>`})}</p>
    <footer class="sticky-actions">
      ${actionButton(t('cancel'),'home','ghost')}
      ${actionButton(t('reset_center'),'reset-calibration','secondary')}
      ${actionButton(t('save_calibration'),'save-calibration')}
    </footer>
  </section>`;
}
function artThemeSetup(){
  const theme=appState.experience.art_theme || 'cartoon';
  return `<section class="art-theme-setup">
    <div><span>${t('artwork_theme')}</span><h2>${theme==='neon'?'Classic Neon':'Playful Cartoon'}</h2><p>${t('artwork_copy')}</p></div>
    <div class="segmented">
      <button class="${theme==='cartoon'?'selected':''}" data-action="art-theme" data-theme="cartoon">Cartoon</button>
      <button class="${theme==='neon'?'selected':''}" data-action="art-theme" data-theme="neon">Classic Neon</button>
    </div>
  </section>`;
}
function soundSetup(){
  const sound=appState.experience.sound || {enabled:false,status:'disabled'};
  const statusLabels={
    disabled:language()==='en'?'OFF':'AUS',
    starting:language()==='en'?'STARTING':'WIRD GESTARTET',
    ready:t('ready'),
    blocked:language()==='en'?'AUTOPLAY BLOCKED':'AUTOPLAY BLOCKIERT',
    unavailable:language()==='en'?'UNAVAILABLE':'NICHT VERFÜGBAR',
  };
  return `<section class="sound-setup ${sound.enabled?'enabled':''}">
    <div><span>${t('projector_sound')}</span><h2>${sound.enabled?t('enabled'):t('disabled')}</h2><p>${t('status')}: <b>${statusLabels[sound.status] || escapeHtml(sound.status)}</b></p></div>
    <div class="sound-setup-actions">
      ${actionButton(sound.enabled?t('sound_off'):t('sound_on'),sound.enabled?'sound-disable':'sound-enable',sound.enabled?'ghost':'primary')}
      ${actionButton(t('test_tone'),'sound-test','secondary',sound.enabled?'':'disabled')}
    </div>
    ${sound.status==='blocked'?`<small>${t('autoplay_help')}</small>`:''}
  </section>`;
}

function renderProjector(){
  const root = $('projectorRoot');
  if(!root) return;
  const screen = appState.experience.screen;
  const modeSlug=appState.experience.game?.game_type || appState.experience.selected_mode || '';
  const currentAmbience=root.querySelector('.game-ambience');
  const preserveAmbience=(
    currentAmbience
    && (screen==='playing' || screen==='game_result')
    && currentAmbience.dataset.mode===modeSlug
  ) ? currentAmbience : null;
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
  const markup=(renderers[screen] || projectorAttract)();
  if(preserveAmbience){
    const template=document.createElement('template');
    template.innerHTML=markup.trim();
    const currentScene=root.firstElementChild;
    const freshScene=template.content.firstElementChild;
    const freshAmbience=freshScene?.querySelector('.game-ambience');
    if(currentScene && freshScene && freshAmbience){
      currentScene.className=freshScene.className;
      currentScene.style.cssText=freshScene.style.cssText;
      preserveAmbience.style.cssText=freshAmbience.style.cssText;
      preserveAmbience.classList.toggle('frozen',freshAmbience.classList.contains('frozen'));
      preserveAmbience.classList.remove('react-hit','react-miss');
      [...currentScene.children].forEach(child=>{
        if(child!==preserveAmbience) child.remove();
      });
      [...freshScene.children].forEach(child=>{
        if(child!==freshAmbience) currentScene.appendChild(child);
      });
      const reaction=appState.projectedEvent?.type;
      if(screen==='playing' && (reaction==='hit' || reaction==='miss')){
        void preserveAmbience.offsetWidth;
        preserveAmbience.classList.add(`react-${reaction}`);
      }
    }else{
      root.innerHTML=markup;
    }
  }else{
    root.innerHTML=markup;
  }
  if(screen === 'playing' || screen === 'game_result' || screen === 'calibration'){
    buildBoard();
    applyCalibration();
    if(screen === 'playing' || screen === 'game_result'){
      const boardEvent = screen === 'game_result'
        ? (appState.projectedEvent || appState.experience.game?.last_event)
        : appState.projectedEvent;
      renderBoardEvent(boardEvent);
    }
  }
}
function projectorBackdrop(mode, inner, className=''){
  const image = modeAsset(mode?.slug || 'countup');
  return `<section class="projector-scene ${className}" style="--scene-image:url('${image}');--accent:${escapeHtml(mode?.accent || '#28e7ff')}"><div class="cinema-shade"></div>${inner}</section>`;
}
function projectorAttract(){
  const title=language()==='en'?'Ready for your session?':'Bereit für eure Session?';
  const copy=language()==='en'?'Start from the Control screen':'Am Control-Screen starten';
  return projectorBackdrop(null,`<div class="projector-center"><div class="projector-logo">◎</div><div class="kicker">SMART DART EXPERIENCE</div><h1>${escapeHtml(title)}</h1><p>${escapeHtml(copy)}</p></div>`,'attract-projector');
}
function projectorPlayers(){
  return projectorBackdrop(null,`<div class="projector-center"><div class="kicker">${t('session_setup')}</div><h1>${t('who_plays')}</h1><p>${language()==='en'?'Choose players on the Control screen.':'Wählt eure Spieler am Control-Screen.'}</p></div>`);
}
function projectorGameSelect(){
  const players = appState.experience.session?.players || [];
  return projectorBackdrop(null,`<div class="projector-center"><div class="kicker">${t('team_ready')}</div><h1>${players.map(player=>escapeHtml(player.name)).join(' · ')}</h1><p>${language()==='en'?'Choose your game mode.':'Wählt jetzt euren Spielmodus.'}</p></div>`);
}
function projectorInstructions(){
  const mode = modeBySlug(appState.experience.selected_mode);
  const instructions=mode.control_legend?.length
    ? controlLegend(mode.control_legend,'projector-guide')
    : `<div class="projector-step-list">${instructionSteps(mode)}</div>`;
  return projectorBackdrop(mode,`<div class="projector-instructions">
    <div><div class="kicker">${escapeHtml(mode.tagline)}</div><h1>${escapeHtml(mode.title)}</h1><p>${escapeHtml(mode.description)}</p></div>
    ${instructions}
  </div>`,'mode-projector');
}
function projectorCountdown(){
  const mode = modeBySlug(appState.experience.selected_mode);
  return projectorBackdrop(mode,`<div class="projector-center"><div id="countdownValue" class="giant-countdown" aria-live="polite">3</div><div class="kicker">GET READY</div></div>`,'countdown-projector');
}

function boardSvg(){
  return `<svg id="dartboardSvg" class="dartboard-svg" viewBox="0 0 500 500" aria-label="Kalibrierte Dartboard-Projektion"></svg>`;
}
function projectorAdvice(game){
  const advice = game.advice;
  if(game.game_type !== 'x01' || !advice || !advice.primary || game.status === 'hold') return '';
  const headline = advice.status === 'checkout'
    ? 'FINISH'
    : advice.status === 'setup'
      ? (language()==='en'?'SET UP':'STELLEN')
      : (language()==='en'?'NEXT DART':'NÄCHSTER WURF');
  const sequence = (advice.sequence || []).map(dart => dart.label).join(' → ');
  return `<aside class="projector-advice ${escapeHtml(advice.status)}"><span>${headline}</span><b>${escapeHtml(advice.primary.label)}</b><small>${escapeHtml(sequence || advice.message || '')}</small></aside>`;
}

function projectorOverlayPrompt(game){
  const overlay = game.overlay;
  if(!overlay?.prompt || game.status === 'hold' || game.game_type === 'x01' || overlay.card || overlay.boss || overlay.cricket || Number.isFinite(overlay.pot)) return '';
  const legend=modeBySlug(game.game_type)?.control_legend;
  if(legend?.length) return controlLegend(legend,'projector-live');
  const prompt=game.game_type==='simon_says' && appState.memoryHidden
    ? (language()==='en'?'Now from memory!':'Jetzt aus dem Kopf!')
    : localText(overlay.prompt);
  const label=game.game_type==='simon_says' && !appState.memoryHidden
    ? (language()==='en'?'MEMORIZE FOR 3 SECONDS':'3 SEKUNDEN MERKEN')
    : (language()==='en'?'TARGET':'ZIEL');
  const detail=overlay.combo?.count ? `Combo ×${overlay.combo.count}` : '';
  const visualLegend=visualLegendMarkup(overlay.visual_legend,'projector-visual-legend');
  return `<aside class="projector-advice arcade"><span>${label}</span><b>${promptMarkup(prompt)}</b>${detail?`<small>${escapeHtml(detail)}</small>`:''}${visualLegend}</aside>`;
}
function projectorModePanel(game){
  const overlay = game.overlay || {};
  if(overlay.panel) return genericPanel(overlay.panel,true);
  if(overlay.cricket?.remaining?.length){
    return `<aside class="projector-mode-panel cricket-panel">
      <span>${localText('NOCH ZU SCHLIESSEN')}</span>
      <div>${overlay.cricket.remaining.map(item=>`<b><i>${escapeHtml(item.label)}</i><em>${'●'.repeat(item.marks)}${'○'.repeat(item.needed)}</em><strong>×${item.needed}</strong></b>`).join('')}</div>
    </aside>`;
  }
  if(Array.isArray(overlay.card)){
    return `<aside class="projector-mode-panel bingo-panel"><span>${language()==='en'?'BINGO CARD':'BINGO-KARTE'}</span><div>${overlay.card.map(cell=>`<b class="${cell.done?'done':''}">${escapeHtml(localText(cell.label))}</b>`).join('')}</div></aside>`;
  }
  if(overlay.boss){
    const hp=Math.max(0,Number(overlay.boss.hp)||0), max=Math.max(1,Number(overlay.boss.max_hp)||1);
    return `<aside class="projector-mode-panel boss-panel"><span>BOSS</span><strong>${hp} HP</strong><i style="--progress:${Math.max(0,Math.min(100,hp/max*100))}%"></i></aside>`;
  }
  if(Number.isFinite(overlay.pot)){
    return `<aside class="projector-mode-panel pot-panel"><span>${language()==='en'?'YOUR POT':'DEIN POT'}</span><strong>${overlay.pot}</strong></aside>`;
  }
  return '';
}
function modeAmbience(mode, event, frozen=false){
  const effect=MODE_AMBIENCE[mode?.slug] || 'sparkles';
  const prop=EFFECT_ASSET_NAMES[effect];
  const reaction=event?.type==='hit' ? 'react-hit' : event?.type==='miss' ? 'react-miss' : '';
  const propStyle=prop ? `;--effect-art:url('${effectAsset(prop)}')` : '';
  return `<div class="game-ambience ${reaction} ${frozen?'frozen':''}" data-mode="${escapeHtml(mode?.slug || '')}" aria-hidden="true" style="--game-art:url('${modeAsset(mode?.slug || 'countup')}')${propStyle}">
    <div class="game-art-backdrop"></div>
    <div class="ambient-vignette"></div>
    <div class="ambient-ribbons"><i></i><i></i></div>
    <div class="ambient-effects effect-${effect}">${Array.from({length:12},()=>'<i></i>').join('')}</div>
  </div>`;
}

function bombExplosion(game,event){
  if(!isExplosionEvent({game},event)) return '';
  const index=BOARD_ORDER.indexOf(Number(event.field));
  const radii={
    double:222,
    triple:129,
    single_inner:78,
    single_outer:176,
    single_bull:0,
    double_bull:0,
  };
  const radius=radii[event.ring];
  if(radius===undefined || (Number(event.field)!==25 && index<0)) return '';
  const [x,y]=Number(event.field)===25
    ? [250,250]
    : polar(250,250,radius,index*18);
  const vectors=[
    [-118,-94],[-42,-142],[45,-132],[124,-78],[148,4],[112,96],
    [35,142],[-50,136],[-126,82],[-151,-6],[-82,-48],[74,58],
  ];
  return `<div class="bomb-explosion" style="--blast-x:${x*2}px;--blast-y:${y*2}px" aria-hidden="true">
    <img src="${effectAsset('mine_explosion')}" alt="">
    <b>BOOM!</b>
    ${vectors.map(([dx,dy],index)=>`<i style="--dx:${dx}px;--dy:${dy}px;--delay:${index%3*25}ms"></i>`).join('')}
  </div>`;
}

function candyOverheat(game,event){
  if(!isCandyOverheatEvent({game},event)) return '';
  return `<div class="candy-overheat" aria-hidden="true">
    <i></i>
    <img src="${effectAsset('candy_overheat')}" alt="">
    <b>${t('overheat')}</b>
    <span>${t('charge_lost')}</span>
  </div>`;
}

function candyFireImpact(game,event,playerId){
  if(!isCandyFireEvent({game},event) || event.target_player_id!==playerId) return '';
  const loss=Math.max(0,Number(event.target_score_loss)||0);
  return `<em class="candy-fire-hit" aria-hidden="true">
    <img src="${effectAsset('candy_overheat')}" alt="">
    <b>${loss>0?`−${loss}`:'TREFFER!'}</b>
  </em>`;
}

function dragonReaction(game,event){
  const effects=['dragon_egg','dragon_scale','dragon_fire'];
  if(game?.game_type!=='dragon_eggs' || !effects.includes(event?.effect)) return '';
  const index=BOARD_ORDER.indexOf(Number(event.field));
  const radii={
    double:222, triple:129, single_inner:78, single_outer:176,
    single_bull:0, double_bull:0,
  };
  const radius=radii[event.ring];
  if(radius===undefined || (Number(event.field)!==25 && index<0)) return '';
  const [x,y]=event.effect==='dragon_fire' || Number(event.field)===25
    ? [250,250]
    : polar(250,250,radius,index*18);
  const asset=event.effect==='dragon_egg'?'egg':event.effect==='dragon_scale'?'dragon_scale':'dragon_fire';
  const label=event.effect==='dragon_egg'
    ? '+30 · EI GEKNACKT!'
    : event.effect==='dragon_scale'
      ? `HITZE ${Math.max(1,Number(event.dragon_heat)||1)}/3`
      : `DRACHENFEUER · −${Math.max(15,Number(event.dragon_fire_penalty)||15)}`;
  return `<div class="dragon-reaction ${escapeHtml(event.effect)}" style="--dragon-x:${x*2}px;--dragon-y:${y*2}px" aria-hidden="true">
    <i></i><img src="${effectAsset(asset)}" alt=""><b>${escapeHtml(label)}</b>
  </div>`;
}

function cookieReaction(game,event){
  if(game?.game_type!=='cookie_monster' || !['cookie_eaten','cookie_board_clear'].includes(event?.effect)) return '';
  const index=BOARD_ORDER.indexOf(Number(event.field));
  const radii={
    double:222, triple:129, single_inner:78, single_outer:176,
    single_bull:0, double_bull:0,
  };
  const radius=radii[event.ring];
  const centered=event.effect==='cookie_board_clear';
  if(!centered && (radius===undefined || index<0)) return '';
  const [x,y]=centered ? [250,250] : polar(250,250,radius,index*18);
  const points=Math.max(0,Number(event.cookie_points)||0);
  const label=centered ? 'BOARD GEPUTZT!' : `+${points} · MAMPF!`;
  return `<div class="cookie-reaction ${escapeHtml(event.effect)}" style="--cookie-x:${x*2}px;--cookie-y:${y*2}px" aria-hidden="true">
    <i></i><img src="${effectAsset('cookie')}" alt=""><b>${escapeHtml(label)}</b>
    ${centered?'<span>NEUE COOKIES!</span>':''}
  </div>`;
}

function projectorPlaying(){
  const game = appState.experience.game;
  const mode = modeBySlug(game.game_type);
  const player = currentPlayer(game) || {};
  const testMode=testModeEnabled();
  const bombImpact=isExplosionEvent({game},appState.projectedEvent);
  const candyOverheatImpact=isCandyOverheatEvent({game},appState.projectedEvent);
  const dragonFireImpact=isDragonEvent({game},appState.projectedEvent,'dragon_fire');
  return `<section class="projection-game themed-game ${testMode?'test-mode':''} ${bombImpact?'bomb-impact':''} ${candyOverheatImpact?'candy-overheat-impact':''} ${dragonFireImpact?'dragon-fire-impact':''}" style="--accent:${escapeHtml(mode?.accent || '#28e7ff')};--candy-art:url('${effectAsset('candy')}')">
    ${modeAmbience(mode,appState.projectedEvent)}
    <div id="projectionPlane" class="projection-plane"><div class="board-stage-shield"></div>${boardSvg()}<div id="boardPulse" class="board-pulse"></div>${bombExplosion(game,appState.projectedEvent)}${candyOverheat(game,appState.projectedEvent)}${dragonReaction(game,appState.projectedEvent)}${cookieReaction(game,appState.projectedEvent)}</div>
    <header class="projection-top"><div><div class="kicker">${escapeHtml(mode?.title || '')} · ${t('round')} ${game.round_number}</div><h1>${escapeHtml(player.name || '')}</h1></div><strong data-score-player="${escapeHtml(player.id || '')}">${player.score ?? 0}</strong></header>
    <footer class="projection-bottom">
      <div class="throw-callout">${game.status==='hold'?t('pull_darts'):escapeHtml(appState.projectedEvent?.label || t('ready'))}</div>
      <div>${game.darts_in_turn}/3 ${t('darts')} · ${game.turn_score} ${t('points_short')}</div>
    </footer>
    ${projectorAdvice(game)}
    ${projectorOverlayPrompt(game)}
    ${projectorModePanel(game)}
    <aside class="projection-roster">${game.players.map(item=>{
      const fireTarget=isCandyFireEvent({game},appState.projectedEvent) && appState.projectedEvent.target_player_id===item.id;
      return `<span class="${item.id===game.current_player_id?'active':''} ${fireTarget?'candy-fire-target':''}"><b>${escapeHtml(item.name)}</b><i data-score-player="${escapeHtml(item.id)}">${item.score}</i>${candyFireImpact(game,appState.projectedEvent,item.id)}</span>`;
    }).join('')}</aside>
    ${testMode?`<div class="projector-test-tools"><b>${t('test_mode')}</b><span>${t('click_segment')}</span><button data-action="test-miss">MISS</button><button class="switch-player" data-action="next-player">${t('switch_player')}</button></div>`:''}
  </section>`;
}
function projectorResult(){
  const game = appState.experience.game;
  const champion = winner();
  const result = resultCopy(game,champion);
  const mode = modeBySlug(game.game_type);
  const lastThrow = game.throws?.at(-1);
  return `<section class="projection-game themed-game result-board" style="--accent:${escapeHtml(mode?.accent || '#28e7ff')}">
    ${modeAmbience(mode,null,true)}
    <div id="projectionPlane" class="projection-plane"><div class="board-stage-shield"></div>${boardSvg()}</div>
    <header class="projection-top result-board-heading">
      <div><div class="kicker">${escapeHtml(mode?.title || '')} · ${t('result')}</div><h1>${t('final_score')}</h1></div>
    </header>
    ${projectorModePanel(game)}
    <aside class="projection-roster result-roster">${game.players.map(item=>`<span class="${(game.winner_ids||[]).includes(item.id)||item.id===game.winner_id?'winner':''}"><b>${escapeHtml(item.name)}</b><i>${item.score}</i></span>`).join('')}</aside>
    <div class="victory-overlay">
      <article class="victory-card ${champion?'':'no-winner'}">
        <div class="winner-crown">${result.icon}</div>
        <div><span>${result.label}</span><h1>${escapeHtml(result.title)}</h1><p>${escapeHtml(localText(game.message || (language()==='en'?'Game complete':'Spiel beendet')))}</p></div>
        <footer>
          <b>${result.points}</b>
          ${lastThrow ? `<small>LETZTER DART · ${escapeHtml(lastThrow.label || 'MISS')}</small>` : ''}
          ${rematchPrompt(true)}
        </footer>
      </article>
    </div>
  </section>`;
}
function projectorSummary(){
  return projectorBackdrop(null,`<div class="projector-summary"><div>${sceneHeader(t('session_complete'),language()==='en'?'What a session!':'Was für eine Runde!',t('highlights'))}</div><div class="projector-stats">${statCards()}</div></div>`);
}
function projectorCalibration(){
  return `<section class="projection-game calibration-projector">
    <div id="projectionPlane" class="projection-plane calibration-plane">${boardSvg()}<span class="calibration-cross">+</span></div>
    <div class="calibration-caption"><b>${language()==='en'?'CALIBRATION':'KALIBRIERUNG'}</b><span>${t('calibration_projector')}</span></div>
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
  buildBoardElement(svg);
}
function buildBoardElement(svg){
  if(!svg || svg.dataset.ready) return;
  svg.dataset.ready='1';
  const rings={double:[210,235],singleOuter:[142,210],triple:[116,142],singleInner:[38,116]};
  let html='<circle class="board-bg" cx="250" cy="250" r="244"/>';
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
  html += '<g id="overlayLabels" class="overlay-labels"></g>';
  svg.innerHTML=html;
}
function ringToZone(ring){
  return ({double:'double',triple:'triple',single_inner:'singleInner',single_outer:'singleOuter',single_bull:'singleBull',double_bull:'doubleBull'})[ring];
}
function renderBoardEvent(event){
  document.querySelectorAll('.seg.hit,.seg.miss-zone,.seg.advice-target,.seg.overlay-target,.seg.overlay-danger,.seg.overlay-bonus,.seg.overlay-owned').forEach(element=>{element.classList.remove('hit','miss-zone','advice-target','overlay-target','overlay-danger','overlay-bonus','overlay-owned'); element.style.removeProperty('--owner-color');});
  document.querySelectorAll('.seg.overlay-zone-covered,.seg.overlay-zone-revealed,.seg.overlay-zone-mine,.seg.overlay-zone-control').forEach(element=>{
    element.classList.remove('overlay-zone-covered','overlay-zone-revealed','overlay-zone-mine','overlay-zone-control');
    element.style.removeProperty('--zone-color');
  });
  const overlay = appState.experience?.game?.overlay;
  const labelLayer=$('overlayLabels');
  if(labelLayer) labelLayer.innerHTML='';
  const paintedProps=new Set();
  const paintLabel = item => {
    if(!labelLayer || (!item.label && !item.icon)) return;
    const index=BOARD_ORDER.indexOf(Number(item.field));
    const radii={double:222,triple:129,single_inner:78,single_outer:176,single_bull:30,double_bull:0};
    const radius=radii[item.ring];
    const isBull=Number(item.field)===25;
    if(radius===undefined || (!isBull && index<0)) return;
    const [x,y]=isBull ? [250,250] : polar(250,250,radius,index*18);
    const asset=overlayIconAsset(item.icon);
    if(!asset){
      labelLayer.insertAdjacentHTML('beforeend',`<text x="${x}" y="${y}" text-anchor="middle" dominant-baseline="middle">${escapeHtml(item.label)}</text>`);
      return;
    }
    const propKey=isBull ? `${item.icon}:bull` : `${item.icon}:${item.ring}:${item.field}`;
    if(paintedProps.has(propKey)) return;
    paintedProps.add(propKey);
    const fieldHit=event?.type==='hit' && Number(event.field)===Number(item.field);
    const propHit=fieldHit && (
      item.icon==='milk'
      || item.icon==='mine'
      || item.match_field
      || String(event.ring)===String(item.ring)
    );
    const variant=/^[a-z_-]+$/.test(String(item.variant || '')) ? item.variant : 'default';
    const width=item.icon==='milk'?26:item.icon==='mine'?35:item.icon==='egg'?26:item.icon==='dragon_scale'?28:30;
    const height=item.icon==='milk'?36:item.icon==='mine'?39:item.icon==='egg'?34:item.icon==='dragon_scale'?34:30;
    const imageY=y-height/2-(item.label?5:0);
    const labelY=y+height/2+4;
    labelLayer.insertAdjacentHTML('beforeend',`<g class="overlay-prop variant-${variant} ${propHit?'overlay-prop-hit':''}">
      <circle class="overlay-prop-halo" cx="${x}" cy="${y-3}" r="${item.icon==='milk'?18:17}" fill="${escapeHtml(item.color || '#e9a23b')}"></circle>
      <image href="${asset}" x="${x-width/2}" y="${imageY}" width="${width}" height="${height}" preserveAspectRatio="xMidYMid meet"></image>
      ${item.label?`<text x="${x}" y="${labelY}" text-anchor="middle" dominant-baseline="middle">${escapeHtml(item.label)}</text>`:''}
    </g>`);
  };
  const paint = (items, cls) => (items || []).forEach(item => {
    const segment = $(boardSegmentId(ringToZone(item.ring), item.field));
    if(segment) segment.classList.add(cls);
    paintLabel(item);
  });
  const paintOwned = (items) => (items || []).forEach(item => {
    const segment = $(boardSegmentId(ringToZone(item.ring), item.field));
    if(segment){ segment.classList.add('overlay-owned'); segment.style.setProperty('--owner-color', item.color || '#28e7ff'); }
  });
  const paintZones = (items) => (items || []).forEach(item => {
    const rings=item.rings || (item.ring ? [item.ring] : []);
    rings.forEach((ring,index)=>{
      const segment=$(boardSegmentId(ringToZone(ring),item.field));
      if(segment){
        segment.classList.add(`overlay-zone-${item.role||'revealed'}`);
        if(item.color) segment.style.setProperty('--zone-color',item.color);
      }
      if(
        (index===0 || item.label_all)
        && (item.icon || (item.label!==undefined && item.label!==''))
      ) paintLabel({...item,ring});
    });
  });
  const game=appState.experience?.game;
  const gameStatus=game?.status;
  const activeTargetsHidden = (
    (gameStatus === 'hold' || gameStatus === 'finished')
    && ['target_rush','lightning_round','simon_says'].includes(game?.game_type)
  ) || (game?.game_type === 'simon_says' && appState.memoryHidden);
  if(
    (gameStatus === 'running' || gameStatus === 'hold' || gameStatus === 'finished')
    && !activeTargetsHidden
  ){
    paint(overlay?.targets, 'overlay-target');
    paint(overlay?.danger, 'overlay-danger');
    paint(overlay?.bonus, 'overlay-bonus');
    paintOwned(overlay?.owned);
    paintZones(overlay?.zones);
  }
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
async function reportProjectorGeometry(){
  if(!isProjector()) return;
  const geometry=`${innerWidth}x${innerHeight}`;
  if(geometry===appState.reportedGeometry) return;
  appState.reportedGeometry=geometry;
  try{
    await action('/api/projector/geometry',{width:innerWidth,height:innerHeight});
  }catch(error){
    appState.reportedGeometry='';
  }
}

async function reportAudioStatus(status){
  if(!isProjector() || status===appState.reportedAudioStatus) return;
  appState.reportedAudioStatus=status;
  try{
    await action('/api/sound/status',{status});
  }catch(error){
    appState.reportedAudioStatus='';
  }
}
function ensureAudio(reportStatus=false){
  if(!appState.audio){
    const AudioContext=window.AudioContext||window.webkitAudioContext;
    if(AudioContext) appState.audio=new AudioContext();
  }
  if(!appState.audio){
    if(reportStatus) reportAudioStatus('unavailable');
    return null;
  }
  if(appState.audio.state==='suspended'){
    appState.audio.resume().then(()=>{
      if(reportStatus) reportAudioStatus(appState.audio.state==='running'?'ready':'blocked');
    }).catch(()=>{ if(reportStatus) reportAudioStatus('blocked'); });
  }else if(reportStatus){
    reportAudioStatus(appState.audio.state==='running'?'ready':'blocked');
  }
  return appState.audio;
}
function tone(frequency,duration=0.12,delay=0,type='sine',gain=0.12){
  if(!isProjector() || !appState.experience?.sound?.enabled) return;
  const context=ensureAudio();
  if(!context || context.state!=='running') return;
  const oscillator=context.createOscillator(), volume=context.createGain();
  oscillator.type=type; oscillator.frequency.value=frequency;
  volume.gain.setValueAtTime(0.0001,context.currentTime+delay);
  volume.gain.exponentialRampToValueAtTime(gain,context.currentTime+delay+0.015);
  volume.gain.exponentialRampToValueAtTime(0.0001,context.currentTime+delay+duration);
  oscillator.connect(volume).connect(context.destination);
  oscillator.start(context.currentTime+delay); oscillator.stop(context.currentTime+delay+duration+0.02);
}
function playEventCue(event,experience){
  if(!isProjector() || !experience.sound?.enabled) return;
  const key=`${event.type}:${event.seq ?? event.action ?? Date.now()}`;
  if(key===appState.lastCueKey) return;
  appState.lastCueKey=key;
  if(event.type==='sound_test'){
    [440,660,880].forEach((frequency,index)=>tone(frequency,.2,index*.1,'triangle',.12));
    return;
  }
  const theme=modeBySlug(experience.game?.game_type)?.sound_theme || 'arena';
  const themeBase={arcade:500,club:390,championship:450,arena:420}[theme] || 420;
  if(event.type==='hit' && isExplosionEvent(experience,event)){
    tone(92,.38,0,'sawtooth',.18);
    tone(54,.52,.04,'sawtooth',.16);
    tone(680,.09,0,'square',.07);
  } else if(event.type==='hit' && isCandyOverheatEvent(experience,event)){
    tone(520,.11,0,'square',.12);
    tone(360,.16,.08,'triangle',.13);
    tone(220,.28,.18,'sawtooth',.1);
    tone(110,.34,.28,'sine',.08);
  } else if(event.type==='hit' && isCandyFireEvent(experience,event)){
    tone(720,.12,0,'square',.08);
    tone(940,.1,.12,'triangle',.07);
    tone(190,.24,.38,'sawtooth',.12);
    tone(520,.16,.42,'square',.06);
  } else if(event.type==='hit' && isDragonEvent(experience,event,'dragon_fire')){
    tone(110,.42,0,'sawtooth',.17);
    tone(72,.58,.05,'sawtooth',.13);
    tone(620,.16,.08,'square',.07);
  } else if(event.type==='hit' && isDragonEvent(experience,event,'dragon_scale')){
    tone(210,.18,0,'triangle',.12);
    tone(145,.24,.1,'sawtooth',.08);
  } else if(event.type==='hit' && isDragonEvent(experience,event,'dragon_egg')){
    tone(680,.12,0,'triangle',.13);
    tone(920,.18,.08,'sine',.1);
    tone(1180,.12,.17,'triangle',.06);
  } else if(event.type==='hit' && isCookieEvent(experience,event,'cookie_board_clear')){
    [520,680,860,1080].forEach((frequency,index)=>tone(frequency,.2,index*.08,'triangle',.1));
  } else if(event.type==='hit' && isCookieEvent(experience,event,'cookie_eaten')){
    tone(460,.09,0,'triangle',.11);
    tone(720,.13,.07,'square',.055);
  } else if(event.type==='hit' && isCookieEvent(experience,event,'cookie_moldy')){
    tone(170,.2,0,'sawtooth',.1);
    tone(105,.28,.1,'triangle',.07);
  } else if(event.type==='hit'){
    const base=event.multiplier===3?themeBase*1.45:event.multiplier===2?themeBase*1.22:event.field===25?themeBase*1.7:themeBase;
    tone(base,.16,0,'triangle',.16); tone(base*1.5,.2,.08,'sine',.1);
    if(theme==='arcade') tone(base*2,.11,.15,'square',.035);
  } else if(event.type==='miss') tone(110,.28,0,'sawtooth',.08);
  else if(event.type==='continue'||event.type==='next_player') tone(330,.14);
  else if(event.type==='hardware_status'&&event.status==='error') tone(95,.4,0,'sawtooth',.06);
  if(experience.game.status==='finished' && experience.game.winner_id){
    [392,523,659,784].forEach((frequency,index)=>tone(frequency,.35,index*.12,'triangle',.13));
  }else if(experience.game.status==='finished'){
    [180,145,110].forEach((frequency,index)=>tone(frequency,.3,index*.12,'sawtooth',.07));
  }
}
function startCountdown(){
  clearInterval(appState.countdownTimer);
  let value=3;
  const tick=()=>{
    const element=$('countdownValue');
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
  if(name==='set-language'){
    appState.selectedLanguage=target.dataset.language==='en'?'en':'de';
    appState.languageTouched=true;
    document.documentElement.lang=appState.selectedLanguage;
    render();
    return;
  }
  if(name==='open-history'){ await openHistory(); return; }
  if(name==='close-history'){
    appState.localView=null;
    renderControl();
    return;
  }
  if(name==='history-overview'){
    appState.history.session=null;
    appState.history.game=null;
    appState.history.replay=null;
    renderControl();
    return;
  }
  if(name==='history-session'){
    try{ await openHistorySession(target.dataset.id); }
    catch(error){ showToast(error.message); }
    return;
  }
  if(name==='history-game'){
    try{ await openHistoryGame(target.dataset.id); }
    catch(error){ showToast(error.message); }
    return;
  }
  if(name==='history-session-back'){
    appState.history.game=null;
    appState.history.replay=null;
    renderControl();
    return;
  }
  if(name==='history-player'){
    const h=appState.history;
    h.playerId=target.dataset.player;
    try{
      [h.heatmap,h.recommendations]=await Promise.all([
        getJson(`/api/statistics/heatmap?player_id=${encodeURIComponent(h.playerId)}${h.includeTest?'&include_test=true':''}`),
        getJson(`/api/training/${encodeURIComponent(h.playerId)}/recommendations`),
      ]);
      renderControl();
    }catch(error){ showToast(error.message); }
    return;
  }
  if(name==='replay-prev' || name==='replay-next'){
    const max=Math.max(0,(appState.history.replay?.events?.length || 1)-1);
    appState.history.replayIndex=Math.max(0,Math.min(max,appState.history.replayIndex+(name==='replay-next'?1:-1)));
    renderControl();
    return;
  }
  if(name==='choose-players'){ await action('/api/navigation/players'); return; }
  if(name==='home'){
    if(appState.localView){ appState.localView=null; renderControl(); return; }
    appState.selectedPlayers.clear(); await action('/api/session/close'); return;
  }
  if(name==='calibrate'){ await action('/api/navigation',{screen:'calibration'}); return; }
  if(name==='toggle-player'){
    appState.selectedPlayers.has(target.dataset.id) ? appState.selectedPlayers.delete(target.dataset.id) : appState.selectedPlayers.add(target.dataset.id);
    renderControl(); return;
  }
  if(name==='start-session'){ await action('/api/session/start',{player_ids:[...appState.selectedPlayers],language:appState.selectedLanguage}); return; }
  if(name==='select-mode'){ await action('/api/game/prepare',{game_type:target.dataset.mode,options:{}}); return; }
  if(name==='set-option'){
    const value=/^-?\d+(\.\d+)?$/.test(target.dataset.value)?Number(target.dataset.value):target.dataset.value;
    const options={...appState.experience.selected_options,[target.dataset.key]:value};
    await action('/api/game/prepare',{game_type:appState.experience.selected_mode,options}); return;
  }
  if(name==='back-games'){ await action('/api/game/next'); return; }
  if(name==='start-game'){ await action('/api/game/start'); return; }
  if(name==='continue'){ await action('/api/continue'); return; }
  if(name==='arm-skip'){ appState.skipArmed=true; renderControl(); return; }
  if(name==='cancel-skip'){ appState.skipArmed=false; renderControl(); return; }
  if(name==='next-player'){ appState.skipArmed=false; await action('/api/next-player'); return; }
  if(name==='undo'){ await action('/api/undo'); return; }
  if(name==='arm-abort'){ appState.abortArmed=true; renderControl(); return; }
  if(name==='cancel-abort'){ appState.abortArmed=false; renderControl(); return; }
  if(name==='abort-game'){
    appState.abortArmed=false;
    await action('/api/game/abort');
    return;
  }
  if(name==='game-action'){
    await action('/api/game/action',{action:target.dataset.gameAction,payload:{}}); return;
  }
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
  if(name==='reset-calibration'){
    await action('/api/calibration/reset');
    return;
  }
  if(name==='sound-enable'){ await action('/api/sound/settings',{enabled:true}); return; }
  if(name==='sound-disable'){ await action('/api/sound/settings',{enabled:false}); return; }
  if(name==='sound-test'){ await action('/api/sound/test'); return; }
  if(name==='art-theme'){ await action('/api/art-theme',{theme:target.dataset.theme}); return; }
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
  if(event.target.id==='replayRange'){
    appState.history.replayIndex=Number(event.target.value);
    renderControl();
    return;
  }
  if(!event.target.matches('[data-corner]')) return;
  const corner=Number(event.target.dataset.corner), axis=event.target.dataset.axis;
  appState.experience.calibration.corners[corner][axis]=Number(event.target.value);
  event.target.nextElementSibling.textContent=`${Math.round(Number(event.target.value)*100)}%`;
});
document.addEventListener('change',event=>{
  if(event.target.matches('[data-action="history-test"]')){
    appState.history.includeTest=event.target.checked;
    appState.history.recommendations=null;
    loadHistory();
    return;
  }
  if(!event.target.matches('[data-corner]')) return;
  action('/api/calibration',appState.experience.calibration);
});
window.addEventListener('resize',()=>{
  if(!isProjector()) return;
  applyCalibration();
  clearTimeout(appState.geometryTimer);
  appState.geometryTimer=setTimeout(reportProjectorGeometry,250);
});
window.addEventListener('load',()=>{ loadBootstrap(); connectWs(); reportProjectorGeometry(); });
