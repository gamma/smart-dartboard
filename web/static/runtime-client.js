(function exposeRuntimeClient(global){
  'use strict';

  const PROTOCOL_VERSION=1;

  class RuntimeClientError extends Error {
    constructor(message,{code='internal',details=null,status=0}={}){
      super(message);
      this.name='RuntimeClientError';
      this.code=code;
      this.details=details;
      this.status=status;
    }
  }

  function commandId(){
    if(global.crypto?.randomUUID) return global.crypto.randomUUID();
    return `web-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  }

  function listenerCall(listener,name,payload){
    if(typeof listener==='function' && name==='onMessage') listener(payload);
    else listener?.[name]?.(payload);
  }

  class HostedRuntimeClient {
    constructor(browserLocation=global.location){
      this.location=browserLocation;
      this.runtimeInstanceId=null;
      this.revision=null;
      this.stopped=false;
      this.socket=null;
      this.retryTimer=null;
      this.resyncing=null;
    }

    async request(path,options){
      const response=await global.fetch(path,options);
      let payload=null;
      try{ payload=await response.json(); }catch(_){ /* handled below */ }
      if(!response.ok){
        throw new RuntimeClientError(
          payload?.message || payload?.detail || `HTTP ${response.status}`,
          {code:payload?.code,details:payload?.details,status:response.status},
        );
      }
      if(payload===null) throw new RuntimeClientError('Runtime returned invalid JSON');
      return payload;
    }

    acceptEnvelope(envelope,{snapshot=false}={}){
      if(envelope?.protocol_version!==PROTOCOL_VERSION || envelope?.kind!=='state'){
        throw new RuntimeClientError('Incompatible runtime state envelope',{code:'incompatible_protocol'});
      }
      const sameRuntime=this.runtimeInstanceId===envelope.runtime_instance_id;
      if(!snapshot && sameRuntime && this.revision!==null && envelope.revision!==this.revision+1){
        if(envelope.revision<=this.revision) return null;
        throw new RuntimeClientError('Runtime revision gap',{code:'revision_gap'});
      }
      this.runtimeInstanceId=envelope.runtime_instance_id;
      this.revision=envelope.revision;
      return envelope;
    }

    async bootstrap(){
      const envelope=await this.request('/api/v2/runtime/bootstrap');
      return this.acceptEnvelope(envelope,{snapshot:true});
    }

    async request_snapshot(){
      const envelope=await this.request('/api/v2/runtime/snapshot');
      return this.acceptEnvelope(envelope,{snapshot:true});
    }

    query(path){ return this.request(path); }

    async dispatch(command,{commandId:stableId=commandId(),expectedRevision=this.revision}={}){
      if(!this.runtimeInstanceId || this.revision===null){
        throw new RuntimeClientError('RuntimeClient must bootstrap before dispatch');
      }
      const envelope={
        protocol_version:PROTOCOL_VERSION,
        command_id:stableId,
        runtime_instance_id:this.runtimeInstanceId,
        expected_revision:expectedRevision,
        command,
      };
      const result=await this.request('/api/v2/runtime/commands',{
        method:'POST',
        headers:{'content-type':'application/json'},
        body:JSON.stringify(envelope),
      });
      if(result.revision>this.revision) this.revision=result.revision;
      return result;
    }

    subscribe(listener){
      this.stopped=false;
      const recover=async()=>{
        if(!this.resyncing){
          this.resyncing=this.request_snapshot()
            .then(envelope=>listenerCall(listener,'onMessage',envelope))
            .finally(()=>{ this.resyncing=null; });
        }
        return this.resyncing;
      };
      const connect=()=>{
        if(this.stopped) return;
        const protocol=this.location.protocol==='https:'?'wss':'ws';
        this.socket=new global.WebSocket(`${protocol}://${this.location.host}/api/v2/runtime/events`);
        this.socket.onopen=()=>listenerCall(listener,'onOpen');
        this.socket.onmessage=async message=>{
          try{
            const envelope=JSON.parse(message.data);
            const runtimeChanged=this.runtimeInstanceId
              && envelope.runtime_instance_id!==this.runtimeInstanceId;
            const gap=this.revision!==null && envelope.revision>this.revision+1;
            if(runtimeChanged || gap){
              await recover();
              return;
            }
            const accepted=this.acceptEnvelope(envelope,{snapshot:this.runtimeInstanceId===null});
            if(accepted) listenerCall(listener,'onMessage',accepted);
          }catch(error){
            listenerCall(listener,'onError',error);
            this.socket?.close();
          }
        };
        this.socket.onclose=()=>{
          listenerCall(listener,'onClose');
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
        throw new RuntimeClientError('Tauri runtime bridge is unavailable');
      }
      this.invoke=tauri.core.invoke;
      this.listen=tauri.event.listen;
      this.unlisten=null;
      this.runtimeInstanceId=null;
      this.revision=null;
    }

    acceptEnvelope(envelope){
      if(envelope?.protocol_version!==PROTOCOL_VERSION || envelope?.kind!=='state'){
        throw new RuntimeClientError('Incompatible native runtime state');
      }
      this.runtimeInstanceId=envelope.runtime_instance_id;
      this.revision=envelope.revision;
      return envelope;
    }

    async bootstrap(){ return this.acceptEnvelope(await this.invoke('runtime_v2_bootstrap')); }
    async request_snapshot(){ return this.acceptEnvelope(await this.invoke('runtime_v2_snapshot')); }
    query(path){ return this.invoke('runtime_v2_query',{path}); }
    async dispatch(command,{commandId:stableId=commandId(),expectedRevision=this.revision}={}){
      if(!this.runtimeInstanceId || this.revision===null){
        throw new RuntimeClientError('RuntimeClient must bootstrap before dispatch');
      }
      const result=await this.invoke('runtime_v2_dispatch',{envelope:{
        protocol_version:PROTOCOL_VERSION,
        command_id:stableId,
        runtime_instance_id:this.runtimeInstanceId,
        expected_revision:expectedRevision,
        command,
      }});
      if(result.revision>this.revision) this.revision=result.revision;
      return result;
    }

    subscribe(listener){
      listenerCall(listener,'onOpen');
      this.listen('runtime-v2-state',async event=>{
        const envelope=event.payload;
        const gap=this.runtimeInstanceId!==envelope.runtime_instance_id
          || (this.revision!==null && envelope.revision>this.revision+1);
        const accepted=gap ? await this.request_snapshot() : this.acceptEnvelope(envelope);
        if(accepted.revision>=this.revision) listenerCall(listener,'onMessage',accepted);
      }).then(unlisten=>{ this.unlisten=unlisten; }).catch(error=>{
        listenerCall(listener,'onError',error);
        listenerCall(listener,'onClose');
      });
      return ()=>this.close();
    }

    close(){
      this.unlisten?.();
      this.unlisten=null;
    }
  }

  class TestRuntimeClient {
    constructor({envelope,dispatch,queries={}}){
      this.envelope=envelope;
      this.dispatchHandler=dispatch;
      this.queries=queries;
      this.listeners=new Set();
    }
    async bootstrap(){ return this.envelope; }
    async request_snapshot(){ return this.envelope; }
    async dispatch(command,options={}){
      return this.dispatchHandler?.(command,options,this) ?? null;
    }
    async query(path){ return this.queries[path]; }
    subscribe(listener){
      this.listeners.add(listener);
      listenerCall(listener,'onOpen');
      return ()=>this.listeners.delete(listener);
    }
    publish(envelope){
      this.envelope=envelope;
      for(const listener of this.listeners) listenerCall(listener,'onMessage',envelope);
    }
    close(){ this.listeners.clear(); }
  }

  class LegacyHostedRuntimeClient {
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
        method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(payload),
      });
    }
    subscribe(listener){
      this.stopped=false;
      const connect=()=>{
        if(this.stopped) return;
        const protocol=this.location.protocol==='https:'?'wss':'ws';
        this.socket=new global.WebSocket(`${protocol}://${this.location.host}/ws`);
        this.socket.onopen=()=>listenerCall(listener,'onOpen');
        this.socket.onmessage=message=>listenerCall(listener,'onMessage',JSON.parse(message.data));
        this.socket.onclose=()=>{
          listenerCall(listener,'onClose');
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

  function createCore(){
    if(global.__TAURI_INTERNALS__ || global.__TAURI__){
      return new TauriRuntimeClient(global.__TAURI__);
    }
    return new HostedRuntimeClient();
  }

  global.SDBRuntimeClient={
    PROTOCOL_VERSION,
    RuntimeClientError,
    HostedRuntimeClient,
    TauriRuntimeClient,
    TestRuntimeClient,
    LegacyHostedRuntimeClient,
    createCore,
    create:()=>new LegacyHostedRuntimeClient(),
  };
})(window);
