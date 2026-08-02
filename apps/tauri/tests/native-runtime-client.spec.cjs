const { test, expect } = require('@playwright/test');
const path = require('node:path');

const runtimeClient = path.resolve(__dirname, '../../../web/static/runtime-client.js');
const experienceClient = path.resolve(__dirname, '../../../web/static/runtime-experience-client.js');

test('experience adapter exposes the authoritative board-rematch countdown', async ({ page }) => {
  await page.goto('about:blank');
  await page.addScriptTag({ path: runtimeClient });
  await page.addScriptTag({ path: experienceClient });

  const result=await page.evaluate(async()=>{
    const now=Date.now();
    const core=new window.SDBRuntimeClient.TestRuntimeClient({
      envelope:{protocol_version:1,kind:'state',runtime_instance_id:'rematch-runtime',revision:7,
        payload:{revision:7,session:{screen:'game_result',players:[],standings:[],
          rematch_armed_until_ms:now+4000},settings:{},effects:[],game:null}},
      queries:{'/api/v2/modes':[],'/api/v2/players':[],'/api/v2/statistics/players':[]},
    });
    const client=new window.SDBRuntimeClient.ExperienceRuntimeClient(core);
    const experience=await client.bootstrap();
    return experience.rematch;
  });

  expect(result.armed).toBe(true);
  expect(result.expires_in_ms).toBeGreaterThan(3000);
  expect(result.expires_in_ms).toBeLessThanOrEqual(4000);
});

test('experience adapter exposes complete v2 history and analytics contracts', async ({ page }) => {
  await page.goto('about:blank');
  await page.addScriptTag({ path: runtimeClient });
  await page.addScriptTag({ path: experienceClient });

  const result = await page.evaluate(async () => {
    const calls=[];
    const core={query:async path=>{
      calls.push(path);
      if(path.includes('/history/sessions/s1')) return {
        session:{id:'s1',started_at:'now'},players:[{id:'p1'}],
        games:[{id:'g1',darts:3}],statistics:[{id:'p1',wins:1}],
      };
      if(path.includes('/history/games/g1/replay')) return {game_id:'g1',events:[{id:1}]};
      if(path.includes('/history/games/g1')) return {
        game:{id:'g1',game_type:'x01'},throws:[{mode_points:40}],events:[{id:1}],
      };
      if(path.includes('/statistics/modes')) return [{game_type:'x01'}];
      if(path.includes('/statistics/heatmap')) return {segments:[{field:20}],total_darts:1};
      if(path.includes('/training/')) return {recommendations:[{field:20}]};
      if(path.includes('/data/export')) return {schema_version:2,database_schema_version:6};
      if(path==='/api/v2/players' || path==='/api/v2/statistics/players') return [];
      throw new Error(`unexpected ${path}`);
    },importData:async archive=>{
      calls.push(`import:${archive.schema_version}`);
      return {players_added:1,sessions_added:0,games_added:0};
    }};
    const client=new window.SDBRuntimeClient.ExperienceRuntimeClient(core);
    return {
      session:await client.query('/api/history/sessions/s1'),
      game:await client.query('/api/history/games/g1'),
      replay:await client.query('/api/history/games/g1/replay'),
      modes:await client.query('/api/statistics/modes?include_test=true'),
      heatmap:await client.query('/api/statistics/heatmap?player_id=p1'),
      training:await client.query('/api/training/p1/recommendations'),
      archive:await client.query('/api/data/export'),
      imported:await client.importData({schema_version:2}),calls,
    };
  });

  expect(result.session).toMatchObject({id:'s1',games:[{id:'g1',darts:3}],statistics:[{wins:1}]});
  expect(result.game).toMatchObject({id:'g1',throws:[{mode_points:40}]});
  expect(result.replay.events).toHaveLength(1);
  expect(result.modes.modes).toEqual([{game_type:'x01'}]);
  expect(result.heatmap.total_darts).toBe(1);
  expect(result.training.recommendations[0].field).toBe(20);
  expect(result.archive).toMatchObject({schema_version:2,database_schema_version:6});
  expect(result.imported.players_added).toBe(1);
  expect(result.calls).toContain('import:2');
  expect(result.calls).toContain('/api/v2/statistics/modes?include_test=true');
});

test('experience adapter consumes each declarative effect once without replaying stale darts', async ({ page }) => {
  await page.goto('about:blank');
  await page.addScriptTag({ path: runtimeClient });
  await page.addScriptTag({ path: experienceClient });

  const result=await page.evaluate(async()=>{
    const hit=seq=>({type:'hit',seq,field:20,ring:'triple',multiplier:3,label:'T20',score:60});
    const envelope=(revision,event,effects=[])=>({
      protocol_version:1,kind:'state',runtime_instance_id:'effect-runtime',revision,
      payload:{revision,session:{screen:'playing',players:[],standings:[]},settings:{},effects,
        game:{game_type:'count_up',state:{players:[],status:'running',last_event:event}}},
    });
    const acknowledgements=[];
    const core=new window.SDBRuntimeClient.TestRuntimeClient({
      envelope:envelope(1,hit(1)),queries:{
        '/api/v2/modes':[],'/api/v2/players':[],'/api/v2/statistics/players':[],
      },acknowledgeEffect:async id=>{ acknowledgements.push(id); return true; },
    });
    const client=new window.SDBRuntimeClient.ExperienceRuntimeClient(core);
    await client.bootstrap();
    const events=[];
    client.subscribe({onMessage:payload=>{
      if(payload.event){
        events.push({type:payload.event.type,seq:payload.event.seq,effectId:payload.event.effect_id});
        payload.event.acknowledge_effect?.();
      }
    }});
    await new Promise(resolve=>queueMicrotask(resolve));
    core.publish(envelope(2,hit(1)));
    const effect={effect_id:'effect:3:sound:controller',revision:3,target:'controller',delivery:'durable',
      kind:{type:'sound',cue:'hit',event:hit(2)}};
    const visual={effect_id:'effect:3:visual:controller',revision:3,target:'controller',delivery:'discardable',
      kind:{type:'visual',cue:'hit',event:hit(2)}};
    core.publish(envelope(3,hit(2),[visual,effect]));
    core.publish(envelope(3,hit(2),[visual,effect]));
    await Promise.resolve();
    return {events,acknowledgements};
  });

  expect(result.events).toEqual([{
    type:'hit',seq:2,effectId:'effect:3:sound:controller',
  }]);
  expect(result.acknowledgements).toEqual([
    'effect:3:visual:controller','effect:3:sound:controller',
  ]);
});

test('native host events update independently from game revisions', async ({ page }) => {
  await page.goto('about:blank');
  await page.addScriptTag({ path: runtimeClient });

  const result = await page.evaluate(async () => {
    const listeners = {};
    const invocations = [];
    let unlistenCount = 0;
    const tauri = {
      core: { invoke: async (command, args) => {
        invocations.push({ command, args });
        return { revision: 2 };
      } },
      event: {
        listen: async (name, listener) => {
          listeners[name] = listener;
          return () => {
            delete listeners[name];
            unlistenCount += 1;
          };
        },
      },
    };
    const client = new window.SDBRuntimeClient.TauriRuntimeClient(tauri);
    client.runtimeInstanceId = 'native-test-runtime';
    client.revision = 1;
    const hostStates = [];
    client.subscribeHost(host => hostStates.push(host));
    await Promise.resolve();
    await client.dispatch({
      type: 'ingest_dart',
      source: 'projector_test',
      event: { type: 'hit', seq: 1, field: 20, ring: 'triple', score: 60 },
    });
    await client.acknowledgeEffect('dart-1:sound:controller');
    await client.importData({ schema_version: 2 });
    listeners['runtime-state']({
      payload: { board: { enabled: true, phase: 'ready' }, external_display_count: 1 },
    });
    client.close();
    return {
      hostStates,
      unlistenCount,
      stillListening: Boolean(listeners['runtime-state']),
      dispatchCommand: invocations[0].command,
      ackCommand: invocations[1].command,
      importCommand: invocations[2].command,
    };
  });

  expect(result.hostStates).toEqual([
    { board: { enabled: true, phase: 'ready' }, external_display_count: 1 },
  ]);
  expect(result.unlistenCount).toBe(1);
  expect(result.stillListening).toBe(false);
  expect(result.dispatchCommand).toBe('runtime_v2_projector_test_event');
  expect(result.ackCommand).toBe('runtime_v2_ack_effect');
  expect(result.importCommand).toBe('runtime_v2_import_data');
});

test('external projector bridge bootstraps queries streams and reports', async ({ page }) => {
  await page.goto('about:blank');
  await page.addScriptTag({ path: runtimeClient });

  const result = await page.evaluate(async () => {
    const subscribers = new Set();
    const dispatched = [];
    const acknowledged = [];
    let payload = {
      envelope: {
        protocol_version: 1,
        kind: 'state',
        runtime_instance_id: 'external-runtime',
        revision: 4,
        payload: { revision: 4, session: {}, settings: {} },
      },
      queries: { '/api/v2/host': { external_display_count: 1 } },
    };
    const bridge = {
      bootstrap: async () => payload.envelope,
      query: async path => payload.queries[path],
      dispatch: async envelope => {
        dispatched.push(envelope);
        return { revision: envelope.expected_revision + 1 };
      },
      acknowledgeEffect: async effectId => {
        acknowledged.push(effectId);
        return true;
      },
      subscribe: listener => {
        subscribers.add(listener);
        return () => subscribers.delete(listener);
      },
    };
    const client = new window.SDBRuntimeClient.ExternalProjectorRuntimeClient(bridge);
    const initial = await client.bootstrap();
    const host = await client.query('/api/v2/host');
    const revisions = [];
    client.subscribe({ onMessage: envelope => revisions.push(envelope.revision) });
    payload = {
      ...payload,
      envelope: { ...payload.envelope, revision: 5, payload: { ...payload.envelope.payload, revision: 5 } },
    };
    for (const subscriber of subscribers) subscriber(payload);
    await Promise.resolve();
    await client.dispatch({
      type: 'report_projector_geometry',
      geometry: { width: 720, height: 448 },
    });
    await client.acknowledgeEffect('dart-4:sound:projector');
    client.close();
    return {
      initialRevision: initial.revision,
      host,
      revisions,
      command: dispatched[0].command,
      remainingSubscribers: subscribers.size,
      acknowledged,
    };
  });

  expect(result.initialRevision).toBe(4);
  expect(result.host).toEqual({ external_display_count: 1 });
  expect(result.revisions).toEqual([5]);
  expect(result.command).toEqual({
    type: 'report_projector_geometry',
    geometry: { width: 720, height: 448 },
  });
  expect(result.remainingSubscribers).toBe(0);
  expect(result.acknowledged).toEqual(['dart-4:sound:projector']);
});
