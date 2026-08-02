import { execFile as execFileCallback } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { promisify } from 'node:util';
import { resolve } from 'node:path';

const execFile=promisify(execFileCallback);
const repository=resolve(import.meta.dirname,'../../..');
const bundle=resolve(repository,
  'apps/tauri/src-tauri/gen/apple/build/arm64-sim/Smart Dartboard.app');
const bundleId='de.gammaproduction.smart-dartboard';
const wait=milliseconds=>new Promise(resolveWait=>setTimeout(resolveWait,milliseconds));
const simctl=async(...arguments_)=>execFile('xcrun',['simctl',...arguments_],{
  cwd:repository,maxBuffer:8*1024*1024,
});

const runtimes=JSON.parse((await simctl('list','runtimes','available','--json')).stdout).runtimes
  .filter(runtime=>runtime.isAvailable && runtime.identifier.includes('iOS')
    && runtime.supportedArchitectures?.includes('arm64'))
  .sort((left,right)=>left.version.localeCompare(right.version,undefined,{numeric:true}));
const runtime=runtimes.at(-1);
if(!runtime) throw new Error('No available arm64 iOS simulator runtime found');
const deviceType=runtime.supportedDeviceTypes?.find(candidate=>candidate.name==='iPad (A16)')
  || runtime.supportedDeviceTypes?.find(candidate=>candidate.productFamily==='iPad');
if(!deviceType) throw new Error(`No iPad device type supports iOS ${runtime.version}`);
const device={
  name:`Smart Dartboard Lifecycle iPad (${runtime.version})`,
  udid:(await simctl('create',`Smart Dartboard Lifecycle ${process.pid}`,
    deviceType.identifier,runtime.identifier)).stdout.trim(),
};

const diagnosticsPath=container=>resolve(container,'Library/Application Support',bundleId,
  'logs/diagnostics.jsonl');
const readDiagnostics=async logPath=>(await readFile(logPath,'utf8')).trim().split('\n')
  .filter(Boolean).map(line=>JSON.parse(line));
const waitForDiagnostics=async(logPath,predicate,label)=>{
  const deadline=Date.now()+10_000;
  let records=[];
  while(Date.now()<deadline){
    records=await readDiagnostics(logPath).catch(()=>[]);
    if(predicate(records)) return records;
    await wait(200);
  }
  throw new Error(`${label} was not observed: ${JSON.stringify(records.slice(-10))}`);
};

try{
  await simctl('boot',device.udid);
  await simctl('bootstatus',device.udid,'-b');
  await simctl('io',device.udid,'screenConfig','--display','external','power','off');
  await simctl('install',device.udid,bundle);
  await simctl('terminate',device.udid,bundleId).catch(()=>{});
  await simctl('launch',device.udid,bundleId);
  const container=(await simctl('get_app_container',device.udid,bundleId,'data')).stdout.trim();
  const logPath=diagnosticsPath(container);
  let records=await waitForDiagnostics(logPath,items=>items.some(record=>
    record.event==='runtime_started' && record.revision===0),'fresh runtime start');
  const started=records.findLastIndex(record=>record.event==='runtime_started');
  const runtimeId=records[started].runtime_instance_id;
  const displayEvents=items=>items.slice(started+1).filter(record=>
    record.event==='external_display_changed' && record.runtime_instance_id===runtimeId);
  records=await waitForDiagnostics(logPath,items=>displayEvents(items).some(record=>
    record.fields?.external_display_count===0),'external display initial disconnect');
  await simctl('io',device.udid,'screenConfig','--display','external','power','on');
  records=await waitForDiagnostics(logPath,items=>displayEvents(items).some(record=>
    record.fields?.external_display_count===1),'external display connect');
  const connected=displayEvents(records).findLastIndex(record=>
    record.fields?.external_display_count===1);
  await simctl('io',device.udid,'screenConfig','--display','external','power','off');
  records=await waitForDiagnostics(logPath,items=>displayEvents(items).slice(connected+1)
    .some(record=>record.fields?.external_display_count===0),'external display disconnect');
  await simctl('launch',device.udid,'com.apple.Preferences');
  await wait(500);
  await simctl('launch',device.udid,bundleId);
  records=await waitForDiagnostics(logPath,items=>{
    const lifecycle=items.slice(started+1).filter(record=>
      record.event==='app_suspended' || record.event==='app_resumed');
    return lifecycle.length>=2 && lifecycle.at(-2).event==='app_suspended'
      && lifecycle.at(-1).event==='app_resumed';
  },'suspend and resume');
  const startedRecord=records[started];
  const lifecycle=records.slice(started+1).filter(record=>
    record.event==='app_suspended' || record.event==='app_resumed');
  if(started<0 || startedRecord.revision!==0 || lifecycle.length<2
    || lifecycle.at(-2).event!=='app_suspended'
    || lifecycle.at(-1).event!=='app_resumed'
    || lifecycle.at(-2).runtime_instance_id!==lifecycle.at(-1).runtime_instance_id
    || lifecycle.at(-2).revision!==lifecycle.at(-1).revision){
    throw new Error(`Lifecycle transition was not preserved: ${JSON.stringify(lifecycle)}`);
  }
  const displays=displayEvents(records);
  if(displays.some(record=>record.revision!==0)){
    throw new Error(`External display changed the runtime: ${JSON.stringify(displays)}`);
  }
  console.log(`iOS lifecycle: fresh ${device.name} connected and disconnected TVOut, suspended and resumed revision 0`);
}finally{
  await simctl('terminate',device.udid,bundleId).catch(()=>{});
  await simctl('shutdown',device.udid).catch(()=>{});
  await simctl('delete',device.udid).catch(()=>{});
}
