(function exposeRuntimeClient(global){
  'use strict';

  class HostedRuntimeClient {
    constructor(browserLocation=global.location){
      this.location=browserLocation;
      this.stopped=false;
      this.socket=null;
      this.retryTimer=null;
    }

    async request(path,options){
      const response=await global.fetch(path,options);
      const payload=await response.json();
      if(!response.ok) throw new Error(payload.detail || `HTTP ${response.status}`);
      return payload;
    }

    bootstrap(){ return this.request('/api/bootstrap'); }
    query(path){ return this.request(path); }
    dispatch(path,payload={}){
      return this.request(path,{
        method:'POST',
        headers:{'content-type':'application/json'},
        body:JSON.stringify(payload),
      });
    }

    subscribe(listener){
      this.stopped=false;
      const connect=()=>{
        if(this.stopped) return;
        const protocol=this.location.protocol==='https:'?'wss':'ws';
        this.socket=new global.WebSocket(`${protocol}://${this.location.host}/ws`);
        this.socket.onopen=()=>listener.onOpen?.();
        this.socket.onmessage=message=>listener.onMessage?.(JSON.parse(message.data));
        this.socket.onclose=()=>{
          listener.onClose?.();
          if(!this.stopped) this.retryTimer=global.setTimeout(connect,1000);
        };
        this.socket.onerror=()=>this.socket?.close();
      };
      connect();
      return ()=>this.close();
    }

    close(){
      this.stopped=true;
      if(this.retryTimer) global.clearTimeout(this.retryTimer);
      this.retryTimer=null;
      this.socket?.close();
      this.socket=null;
    }
  }

  class TauriRuntimeClient {
    constructor(tauri=global.__TAURI__){
      if(!tauri?.core?.invoke || !tauri?.event?.listen){
        throw new Error('Tauri runtime bridge is unavailable');
      }
      this.invoke=tauri.core.invoke;
      this.listen=tauri.event.listen;
      this.unlisten=null;
    }

    bootstrap(){ return this.invoke('runtime_bootstrap'); }
    query(path){ return this.invoke('runtime_query',{path}); }
    dispatch(path,payload={}){
      return this.invoke('runtime_dispatch',{path,payload});
    }

    subscribe(listener){
      listener.onOpen?.();
      this.listen('runtime-state',event=>listener.onMessage?.(event.payload))
        .then(unlisten=>{ this.unlisten=unlisten; })
        .catch(error=>{
          listener.onClose?.(error);
        });
      return ()=>this.close();
    }

    close(){
      this.unlisten?.();
      this.unlisten=null;
    }
  }

  function create(){
    if(global.__SDB_RUNTIME_CLIENT__) return global.__SDB_RUNTIME_CLIENT__;
    if(global.__TAURI_INTERNALS__ || global.__TAURI__){
      return new TauriRuntimeClient(global.__TAURI__);
    }
    return new HostedRuntimeClient();
  }

  global.SDBRuntimeClient={HostedRuntimeClient,TauriRuntimeClient,create};
})(window);
