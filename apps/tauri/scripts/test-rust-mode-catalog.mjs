import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { spawn } from 'node:child_process';
import { webkit } from 'playwright';

const repository=resolve(import.meta.dirname,'../../..');
const dataDirectory=await mkdtemp(`${tmpdir()}/sdb-rust-modes-`);
const port='18083';
const origin=`http://127.0.0.1:${port}`;
const server=spawn('cargo',['run','-p','sdb-server'],{
  cwd:repository,
  env:{...process.env,SDB_BIND:'127.0.0.1',SDB_PORT:port,SDB_ENABLE_BLE:'0',
    SDB_ALLOW_TEST_EVENTS:'0',SDB_DATA_DIR:dataDirectory,SDB_WEB_DIR:resolve(repository,'web')},
  stdio:['ignore','pipe','pipe'],
});
let serverOutput='';
server.stdout.on('data',chunk=>{ serverOutput+=chunk; });
server.stderr.on('data',chunk=>{ serverOutput+=chunk; });

async function waitUntilReady(){
  for(let attempt=0;attempt<120;attempt++){
    if(server.exitCode!==null) throw new Error(`Rust mode server exited early\n${serverOutput}`);
    try{
      const response=await fetch(`${origin}/api/v2/health`);
      if(response.ok) return;
    }catch(_){ /* build or startup still in progress */ }
    await new Promise(resolveWait=>setTimeout(resolveWait,250));
  }
  throw new Error(`Rust mode server did not become ready\n${serverOutput}`);
}

let browser;
try{
  await waitUntilReady();
  const modes=await (await fetch(`${origin}/api/v2/modes`)).json();
  if(modes.length!==24) throw new Error(`Expected 24 modes, received ${modes.length}`);
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
  await control.getByRole('button',{name:'Session starten'}).click();
  for(const name of ['Ada','Bob']){
    await control.locator('input[name="name"]').fill(name);
    await control.getByRole('button',{name:'Spieler anlegen'}).click();
    await control.waitForFunction(playerName=>appState.experience?.players
      ?.some(player=>player.name===playerName),name);
  }
  await control.locator('[data-action="start-session"]').click();
  await control.waitForSelector('.mode-grid');

  const covered=[];
  for(const mode of modes){
    await control.locator(`[data-mode="${mode.slug}"]`).click();
    await Promise.all([
      control.waitForFunction(slug=>appState.experience?.screen==='instructions'
        && appState.experience?.selected_mode===slug,mode.slug),
      projector.waitForFunction(slug=>appState.experience?.screen==='instructions'
        && appState.experience?.selected_mode===slug,mode.slug),
    ]);
    await control.locator('.instruction-control h1').filter({hasText:mode.title}).waitFor();
    await projector.locator('.mode-projector h1').filter({hasText:mode.title}).waitFor();
    await control.locator('.instruction-list').getByText(mode.instructions[0].title).first().waitFor();
    const projectorGuide=mode.control_legend?.[0]?.label || mode.instructions[0].title;
    await projector.locator('.projector-instruction-content')
      .getByText(projectorGuide).first().waitFor();

    await control.locator('[data-action="start-game"]').click();
    await control.waitForFunction(slug=>appState.experience?.screen==='countdown'
      && appState.experience?.game?.game_type===slug,mode.slug);
    await control.evaluate(async()=>{ await action('/api/game/live'); });
    await Promise.all([
      control.waitForSelector('.play-control'),
      projector.waitForSelector('.projection-game'),
    ]);
    const liveMode=await projector.evaluate(()=>appState.experience?.game?.game_type);
    if(liveMode!==mode.slug) throw new Error(`${mode.slug} started as ${liveMode}`);
    await control.evaluate(async()=>{ await action('/api/game/abort'); });
    await Promise.all([
      control.waitForSelector('.mode-grid'),
      projector.waitForFunction(()=>appState.experience?.screen==='game_select'),
    ]);
    covered.push(mode.slug);
  }

  const standings=await control.evaluate(()=>appState.experience.session_statistics);
  if(standings.some(player=>player.games!==0 || player.wins!==0 || player.session_points!==0)){
    throw new Error(`Aborted catalog games changed standings: ${JSON.stringify(standings)}`);
  }
  if(browserErrors.length) throw new Error(`Browser errors:\n${browserErrors.join('\n')}`);
  console.log(`Rust mode catalog: ${covered.length} cards, dual-screen instructions, starts and unscored aborts passed in WebKit`);
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
