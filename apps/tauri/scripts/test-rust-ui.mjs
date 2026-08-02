import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { spawn } from 'node:child_process';
import { webkit } from 'playwright';

const repository=resolve(import.meta.dirname,'../../..');
const dataDirectory=await mkdtemp(`${tmpdir()}/sdb-rust-ui-`);
const port='18082';
const origin=`http://127.0.0.1:${port}`;
const server=spawn('cargo',['run','-p','sdb-server'],{
  cwd:repository,
  env:{...process.env,SDB_BIND:'127.0.0.1',SDB_PORT:port,SDB_ENABLE_BLE:'0',
    SDB_ALLOW_TEST_EVENTS:'1',SDB_DATA_DIR:dataDirectory,SDB_WEB_DIR:resolve(repository,'web')},
  stdio:['ignore','pipe','pipe'],
});
let serverOutput='';
server.stdout.on('data',chunk=>{ serverOutput+=chunk; });
server.stderr.on('data',chunk=>{ serverOutput+=chunk; });

async function waitUntilReady(){
  for(let attempt=0;attempt<120;attempt++){
    if(server.exitCode!==null) throw new Error(`Rust UI server exited early\n${serverOutput}`);
    try{
      const response=await fetch(`${origin}/api/v2/health`);
      if(response.ok) return;
    }catch(_){ /* build or startup still in progress */ }
    await new Promise(resolveWait=>setTimeout(resolveWait,250));
  }
  throw new Error(`Rust UI server did not become ready\n${serverOutput}`);
}

async function waitForEffectsDrained(){
  for(let attempt=0;attempt<40;attempt++){
    const state=await (await fetch(`${origin}/api/v2/runtime/bootstrap`)).json();
    if((state.payload?.effects || []).length===0) return;
    await new Promise(resolveWait=>setTimeout(resolveWait,50));
  }
  throw new Error('Committed platform effects were not acknowledged');
}

let browser;
try{
  await waitUntilReady();
  browser=await webkit.launch({headless:true});
  const context=await browser.newContext({viewport:{width:1366,height:900}});
  const control=await context.newPage();
  const projector=await context.newPage();
  const browserErrors=[];
  for(const page of [control,projector]){
    page.on('console',message=>{
      if(message.type()==='error') browserErrors.push(message.text());
    });
    page.on('pageerror',error=>browserErrors.push(error.message));
  }
  await Promise.all([control.goto(`${origin}/control`),projector.goto(`${origin}/projector`)]);
  await control.getByRole('button',{name:'Einstellungen'}).click();
  await control.locator('[data-action="art-theme"][data-theme="neon"]').click();
  await projector.waitForFunction(()=>appState.experience?.art_theme==='neon');
  await control.locator('[data-action="calibrate"]').click();
  await projector.waitForSelector('.calibration-projector');
  await control.locator('[data-action="reset-calibration"]').click();
  await projector.waitForFunction(()=>{
    const settings=appState.experience;
    const [topLeft,,bottomRight]=settings?.calibration?.corners || [];
    return settings?.projector_geometry?.width===1366
      && settings?.projector_geometry?.height===900
      && Math.abs(topLeft?.y-.05)<1e-9
      && Math.abs(bottomRight?.y-.95)<1e-9;
  });
  await control.locator('[data-action="close-calibration"]').click();
  await projector.waitForSelector('.attract-projector');
  await control.locator('[data-action="sound-enable"]').click();
  await projector.waitForFunction(()=>appState.experience?.sound?.enabled===true
    && appState.experience?.sound?.status!=='starting');
  const soundRevision=await projector.evaluate(()=>appState.experience.revision);
  await control.waitForFunction(revision=>appState.experience?.revision>=revision,soundRevision);
  await control.locator('[data-action="sound-test"]').click();
  await projector.waitForFunction(()=>Boolean(appState.experience?.sound?.last_test_id));
  await waitForEffectsDrained();
  await Promise.all([control.reload(),projector.reload()]);
  await Promise.all([
    control.waitForFunction(()=>appState.experience?.art_theme==='neon'),
    projector.waitForFunction(()=>appState.experience?.art_theme==='neon'),
  ]);
  await control.getByRole('button',{name:'Session starten'}).click();
  await control.locator('input[name="name"]').fill('Ada');
  await control.getByRole('button',{name:'Spieler anlegen'}).click();
  const sessionStart=control.locator('[data-action="start-session"]');
  await sessionStart.waitFor({state:'visible'});
  await control.waitForFunction(()=>!document.querySelector('[data-action="start-session"]')?.disabled);
  await sessionStart.click();
  await control.waitForFunction(()=>document.querySelectorAll('.mode-card').length===24);
  if(await control.locator('.mode-card').count()!==24) throw new Error('Expected 24 mode cards');
  await control.locator('[data-mode="countup"]').click();
  await control.locator('[data-action="start-game"]').click();
  await control.waitForSelector('.play-control',{timeout:8000});
  await projector.waitForSelector('.projection-game',{timeout:8000});
  await projector.locator('#seg-triple-20').click({force:true});
  await control.waitForFunction(()=>document.querySelector('.score-row strong')?.textContent==='60');
  await waitForEffectsDrained();
  await control.evaluate(async()=>{
    await action('/api/game/abort');
    await action('/api/game/prepare',{
      game_type:'x01',options:{start_score:40,out_rule:'double'},
    });
    const playerId=appState.experience.session.players[0].id;
    await action('/api/game/starter',{mode:'manual',player_id:playerId});
    await action('/api/game/start');
    await action('/api/game/live');
    await action('/api/throw/manual',{
      type:'hit',seq:2,field:20,ring:'double',multiplier:2,label:'D20',score:40,
    });
  });
  await control.waitForSelector('.result-control');
  await control.locator('[data-action="next-game"]').click();
  await control.waitForSelector('.mode-grid');
  await control.locator('[data-action="open-history"]').click();
  await control.waitForSelector('.history-dashboard');
  await control.waitForFunction(()=>{
    const history=appState.history;
    return history.players.length===1 && history.sessions.length===1
      && history.modes.some(mode=>mode.game_type==='x01' && mode.finished===1)
      && history.heatmap?.total_darts===1;
  });
  await control.locator('[data-action="history-session"]').click();
  await control.waitForSelector('.history-detail');
  await control.locator('[data-action="history-game"]').last().click();
  await control.waitForSelector('.replay-view');
  await control.waitForFunction(()=>appState.history.game?.throws?.[0]?.mode_points!==undefined);
  const archive=await (await fetch(`${origin}/api/v2/data/export`)).json();
  if(archive.sessions.length!==1 || archive.games.length!==2){
    throw new Error('Expected portable export with one session and two games');
  }
  if(browserErrors.length) throw new Error(`Browser errors:\n${browserErrors.join('\n')}`);
  console.log('Rust Runtime UI: setup, 24 modes, effects, synchronized play, analytics, history, replay and export passed in WebKit');
}finally{
  await browser?.close();
  server.kill('SIGTERM');
  await Promise.race([
    new Promise(resolveExit=>server.once('exit',resolveExit)),
    new Promise(resolveTimeout=>setTimeout(resolveTimeout,2000)),
  ]);
  if(server.exitCode===null) server.kill('SIGKILL');
  await rm(dataDirectory,{recursive:true,force:true});
}
