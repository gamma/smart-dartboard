(async()=>{
  'use strict';
  try{
    const state=await window.__TAURI__.core.invoke('runtime_query');
    const destination=state.app_role==='companion_projector'
      ? '/native.html?role=control'
      : '/control.html';
    location.replace(destination);
  }catch(error){
    document.body.textContent=`Native Runtime konnte nicht gestartet werden: ${String(error)}`;
  }
})();
