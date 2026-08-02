(function exposeExperienceRuntimeClient(global){
  'use strict';

  const api=global.SDBRuntimeClient;
  if(!api) throw new Error('SDBRuntimeClient must be loaded first');

  const DEFAULT_CALIBRATION={
    corners:[
      {x:.247,y:.05},{x:.753,y:.05},{x:.753,y:.95},{x:.247,y:.95},
    ],
    scale:1,offset_x:0,offset_y:0,
  };
  const DEFAULT_CONFIG={
    calibration:DEFAULT_CALIBRATION,
    projector_geometry:{width:1600,height:900},
    sound:{enabled:false,output:'projector',status:'disabled'},
    art_theme:'cartoon',ui_language:'de',correction_lock:{active:false},
  };

  function clone(value){ return JSON.parse(JSON.stringify(value)); }
  function uuid(){
    if(global.crypto?.randomUUID) return global.crypto.randomUUID();
    return `web-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  }
  function readConfig(){
    try{
      const stored=JSON.parse(global.localStorage.getItem('sdb-runtime-v2-config') || 'null');
      return {...clone(DEFAULT_CONFIG),...(stored || {})};
    }catch(_){ return clone(DEFAULT_CONFIG); }
  }
  function saveConfig(config){
    try{ global.localStorage.setItem('sdb-runtime-v2-config',JSON.stringify(config)); }catch(_){}
  }
  function eventRecord(record){
    return {...record.event,action_id:record.action_id,player_id:record.player_id,
      round_number:record.round_number,dart_in_turn:record.dart_in_turn,
      outcome:record.outcome};
  }
  function defaultOptions(mode){
    return Object.fromEntries((mode?.options || []).map(option=>[option.key,option.default]));
  }
  function playerStats(player,standing={}){
    const games=Number(standing.games || 0), wins=Number(standing.wins || 0);
    return {...player,games,wins,session_points:Number(standing.session_points || 0),
      darts:0,total_points:0,best_dart:0,misses:0,three_dart_average:0,
      win_rate:games?Math.round(wins/games*1000)/10:0};
  }

  class ExperienceRuntimeClient {
    constructor(core,{health={}}={}){
      this.core=core;
      this.health=health;
      this.envelope=null;
      this.modes=[];
      this.profiles=[];
      this.statistics=[];
      this.config=readConfig();
      this.localScreen=null;
      this.listener=null;
      this.unsubscribe=null;
    }

    async bootstrap(){
      const [envelope,modes,profiles,statistics]=await Promise.all([
        this.core.bootstrap(),
        this.core.query('/api/v2/modes'),
        this.core.query('/api/v2/players'),
        this.core.query('/api/v2/statistics/players'),
      ]);
      this.envelope=envelope;
      this.modes=modes;
      this.profiles=profiles;
      this.statistics=statistics;
      return this.experience();
    }

    sessionState(){ return this.envelope?.payload?.session || {}; }

    normalizeGame(){
      const wrapped=this.envelope?.payload?.game;
      if(!wrapped) return null;
      const state=wrapped.state || {};
      const gameType=wrapped.game_type==='registered'
        ? state.game_type
        : wrapped.game_type==='count_up'?'countup':wrapped.game_type;
      const profileById=new Map([
        ...this.profiles,...(this.sessionState().players || []),
      ].map(player=>[player.id,player]));
      const players=(state.players || []).map((player,index)=>({
        avatar:'comet',color:['#28e7ff','#ffb52b','#3dff91','#ff4f79'][index%4],
        ...(profileById.get(player.id) || {}),...player,
      }));
      const editable=(state.editable_darts || []).map(eventRecord);
      const current=players[state.current_player_index] || players[0] || null;
      const lastEvent=state.last_event || editable.at(-1) || null;
      return {
        ...state,
        game_type:gameType,
        players,
        current_player_id:current?.id || null,
        overlay:state.overlay || {},
        options:state.options || this.sessionState().prepared_game?.options || {},
        last_event:lastEvent,
        throws:editable,
        cricket_targets:gameType==='cricket'?[20,19,18,17,16,15,25]:undefined,
      };
    }

    editableTurns(game){
      if(!game) return [];
      const darts=game.throws || [];
      const player=game.players?.find(item=>item.id===game.current_player_id);
      const current=darts.filter(item=>
        item.player_id===game.current_player_id && item.round_number===game.round_number);
      return [{
        current:true,
        player_id:game.current_player_id,
        player_name:player?.name || '',
        round_number:game.round_number || 1,
        darts:current,
        can_add:game.status==='running' && current.length<3,
      }];
    }

    experience(){
      const snapshot=this.envelope?.payload || {revision:0,session:{}};
      const session=snapshot.session || {};
      const game=this.normalizeGame();
      const prepared=session.prepared_game;
      const selectedMode=prepared?.game_type || game?.game_type || null;
      const mode=this.modes.find(item=>item.slug===selectedMode);
      const standings=(session.standings || []).map(standing=>{
        const player=(session.players || []).find(item=>item.id===standing.player_id)
          || this.profiles.find(item=>item.id===standing.player_id)
          || {id:standing.player_id,name:standing.player_id,avatar:'comet',color:'#28e7ff'};
        return playerStats(player,standing);
      });
      return {
        screen:this.localScreen || session.screen || 'attract',
        session:session.session_id?{
          id:session.session_id,status:session.session_status,players:session.players || [],
        }:null,
        game_id:session.game_id || null,
        selected_mode:selectedMode,
        selected_options:{...defaultOptions(mode),...(prepared?.options || {})},
        starter:{
          player_id:session.selected_starter_id || null,
          default_player_id:session.default_starter_id || null,
          selection:session.starter_selection || 'rotation',
        },
        game,
        editable_turns:this.editableTurns(game),
        modes:this.modes,
        players:this.profiles,
        statistics:this.statistics,
        session_statistics:standings,
        ...this.config,
        hardware:{enabled:false,status:'disabled',test_events:Boolean(this.health.test_events)},
        rematch:{armed:false,expires_in_ms:0},
        runtime_instance_id:this.envelope?.runtime_instance_id,
        revision:this.envelope?.revision ?? snapshot.revision ?? 0,
      };
    }

    publish(event){
      this.listener?.onMessage?.({
        server_instance:this.envelope?.runtime_instance_id,
        experience:this.experience(),
        event,
      });
    }

    async refresh({publish=true,event}={}){
      this.envelope=await this.core.request_snapshot();
      if(publish) this.publish(event);
      return this.experience();
    }

    async command(command,event){
      await this.core.dispatch(command);
      this.localScreen=null;
      return this.refresh({event});
    }

    async dispatch(path,payload={}){
      if(path==='/api/navigation/players'){
        this.localScreen='players'; this.publish({type:'navigation',screen:'players'});
        return this.experience();
      }
      if(path==='/api/navigation'){
        this.localScreen=payload.screen || 'attract';
        this.publish({type:'navigation',screen:this.localScreen});
        return this.experience();
      }
      if(path==='/api/players'){
        const player={id:uuid(),name:String(payload.name || '').trim(),
          avatar:String(payload.avatar || 'comet'),color:String(payload.color || '#28e7ff')};
        await this.core.dispatch({type:'create_player',player});
        this.profiles=await this.core.query('/api/v2/players');
        await this.refresh({publish:false});
        this.publish({type:'player_created',player_id:player.id});
        return this.profiles.find(item=>item.id===player.id) || player;
      }
      if(path==='/api/session/start'){
        const selected=new Set(payload.player_ids || []);
        const players=this.profiles.filter(player=>selected.has(player.id));
        return this.command({type:'start_session',session_id:uuid(),players},{type:'session_started'});
      }
      if(path==='/api/session/close'){
        if(!this.sessionState().session_id){
          this.localScreen='attract'; this.publish({type:'session_closed'});
          return this.experience();
        }
        return this.command({type:'close_session'},{type:'session_closed'});
      }
      if(path==='/api/game/prepare'){
        const mode=this.modes.find(item=>item.slug===payload.game_type);
        const options={...defaultOptions(mode),...(payload.options || {})};
        return this.command({type:'prepare_game',game_type:payload.game_type,options},{type:'game_prepared'});
      }
      if(path==='/api/game/starter'){
        const sessionPlayers=this.sessionState().players || [];
        const random=payload.mode==='random';
        const playerId=random
          ? sessionPlayers[Math.floor(Math.random()*sessionPlayers.length)]?.id
          : payload.player_id;
        return this.command({type:'select_starter',player_id:playerId,
          selection:random?'random':'manual'},{type:'starter_selected'});
      }
      if(path==='/api/game/start'){
        return this.command({type:'start_prepared_game',game_id:uuid()},{type:'game_started'});
      }
      if(path==='/api/game/live') return this.command({type:'mark_game_playing'},{type:'game_live'});
      if(path==='/api/game/next'){
        const type=this.sessionState().screen==='instructions'?'cancel_prepared_game':'next_game';
        return this.command({type},{type:'game_selection'});
      }
      if(path==='/api/game/abort') return this.command({type:'abort_game'},{type:'game_aborted'});
      if(path==='/api/session/end') return this.command({type:'end_session'},{type:'session_ended'});
      if(path==='/api/continue') return this.command({type:'continue_turn'},{type:'continue'});
      if(path==='/api/next-player') return this.command({type:'next_player'},{type:'next_player'});
      if(path==='/api/undo') return this.command({type:'undo'},{type:'undo'});
      if(path==='/api/game/action') return this.command({type:'game_action',action:payload.action,
        payload:payload.payload || {}},{type:'game_action'});
      if(path==='/api/event' || path==='/api/throw/manual'){
        return this.command({type:'ingest_dart',event:payload,
          source:path==='/api/event'?'projector_test':'manual_correction'},payload);
      }
      if(path==='/api/throw/correct') return this.command({type:'correct_dart',
        action_id:Number(payload.action_id),replacement:payload.event,
        source:'manual_correction'},{...payload.event,type:'correction'});
      if(path==='/api/throw/delete') return this.command({type:'delete_dart',
        action_id:Number(payload.action_id)},{type:'throw_deleted'});
      if(path==='/api/correction/lock'){
        this.config.correction_lock={active:Boolean(payload.enabled)};
        saveConfig(this.config); this.publish({type:'correction_lock'}); return this.experience();
      }
      if(path==='/api/calibration') this.config.calibration=clone(payload);
      else if(path==='/api/calibration/reset') this.config.calibration=clone(DEFAULT_CALIBRATION);
      else if(path==='/api/projector/geometry') this.config.projector_geometry=clone(payload);
      else if(path==='/api/sound/settings') this.config.sound={...payload,
        status:payload.enabled?'ready':'disabled'};
      else if(path==='/api/sound/status') this.config.sound={...this.config.sound,status:payload.status};
      else if(path==='/api/art-theme') this.config.art_theme=payload.theme;
      else if(path==='/api/ui/language') this.config.ui_language=payload.language;
      else if(path==='/api/sound/test') return this.experience();
      else throw new Error(`Unsupported Rust UI action: ${path}`);
      saveConfig(this.config); this.publish({type:'settings'}); return this.experience();
    }

    async query(path){
      if(path.startsWith('/api/history/sessions/')){
        return this.core.query(path.replace('/api/history','/api/v2/history'));
      }
      if(path.startsWith('/api/history/sessions')){
        return {sessions:await this.core.query(path.replace('/api/history','/api/v2/history'))};
      }
      if(path.startsWith('/api/history/games/')){
        return this.core.query(path.replace('/api/history','/api/v2/history'));
      }
      if(path.startsWith('/api/statistics/players')){
        return {players:await this.core.query('/api/v2/statistics/players')};
      }
      if(path.startsWith('/api/statistics/modes')) return {modes:[]};
      if(path.startsWith('/api/statistics/heatmap')){
        return {segments:[],board_hits:0,misses:0,total_darts:0};
      }
      if(path.startsWith('/api/training/')) return {recommendations:[]};
      throw new Error(`Unsupported Rust UI query: ${path}`);
    }

    subscribe(listener){
      this.listener=listener;
      this.unsubscribe=this.core.subscribe({
        onOpen:()=>listener.onOpen?.(),
        onClose:()=>listener.onClose?.(),
        onError:error=>listener.onClose?.(error),
        onMessage:envelope=>{
          this.envelope=envelope;
          this.publish(envelope.payload?.game?.state?.last_event || undefined);
        },
      });
      return ()=>this.close();
    }

    close(){ this.unsubscribe?.(); this.unsubscribe=null; this.core.close(); }
  }

  class AutoRuntimeClient {
    constructor(){ this.delegate=null; this.pendingListener=null; }
    async bootstrap(){
      const core=api.createCore();
      try{
        let health={};
        if(core instanceof api.HostedRuntimeClient){
          health=await core.query('/api/health');
          if(health?.protocol_version!==api.PROTOCOL_VERSION){
            core.close();
            this.delegate=new api.LegacyHostedRuntimeClient();
            if(this.pendingListener) this.delegate.subscribe(this.pendingListener);
            return this.delegate.bootstrap();
          }
        }
        const experience=new ExperienceRuntimeClient(core,{health});
        const state=await experience.bootstrap();
        this.delegate=experience;
        if(this.pendingListener) experience.subscribe(this.pendingListener);
        return state;
      }catch(error){
        core.close();
        if(error?.status!==404 || global.__TAURI_INTERNALS__ || global.__TAURI__) throw error;
        this.delegate=new api.LegacyHostedRuntimeClient();
        if(this.pendingListener) this.delegate.subscribe(this.pendingListener);
        return this.delegate.bootstrap();
      }
    }
    query(path){ return this.delegate.query(path); }
    dispatch(path,payload={}){ return this.delegate.dispatch(path,payload); }
    subscribe(listener){
      if(this.delegate) return this.delegate.subscribe(listener);
      this.pendingListener=listener;
      return ()=>{ this.pendingListener=null; this.delegate?.close(); };
    }
    close(){ this.delegate?.close(); }
  }

  api.ExperienceRuntimeClient=ExperienceRuntimeClient;
  api.AutoRuntimeClient=AutoRuntimeClient;
  api.create=()=>global.__SDB_RUNTIME_CLIENT__ || new AutoRuntimeClient();
})(window);
