const { test, expect } = require('@playwright/test');
const path = require('node:path');

const runtimeClient = path.resolve(__dirname, '../../../web/static/runtime-client.js');
const experienceClient = path.resolve(__dirname, '../../../web/static/runtime-experience-client.js');

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
      throw new Error(`unexpected ${path}`);
    }};
    const client=new window.SDBRuntimeClient.ExperienceRuntimeClient(core);
    return {
      session:await client.query('/api/history/sessions/s1'),
      game:await client.query('/api/history/games/g1'),
      replay:await client.query('/api/history/games/g1/replay'),
      modes:await client.query('/api/statistics/modes?include_test=true'),
      heatmap:await client.query('/api/statistics/heatmap?player_id=p1'),
      training:await client.query('/api/training/p1/recommendations'),
      archive:await client.query('/api/data/export'),calls,
    };
  });

  expect(result.session).toMatchObject({id:'s1',games:[{id:'g1',darts:3}],statistics:[{wins:1}]});
  expect(result.game).toMatchObject({id:'g1',throws:[{mode_points:40}]});
  expect(result.replay.events).toHaveLength(1);
  expect(result.modes.modes).toEqual([{game_type:'x01'}]);
  expect(result.heatmap.total_darts).toBe(1);
  expect(result.training.recommendations[0].field).toBe(20);
  expect(result.archive).toMatchObject({schema_version:2,database_schema_version:6});
  expect(result.calls).toContain('/api/v2/statistics/modes?include_test=true');
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
    listeners['runtime-state']({
      payload: { board: { enabled: true, phase: 'ready' }, external_display_count: 1 },
    });
    client.close();
    return {
      hostStates,
      unlistenCount,
      stillListening: Boolean(listeners['runtime-state']),
      dispatchCommand: invocations[0].command,
    };
  });

  expect(result.hostStates).toEqual([
    { board: { enabled: true, phase: 'ready' }, external_display_count: 1 },
  ]);
  expect(result.unlistenCount).toBe(1);
  expect(result.stillListening).toBe(false);
  expect(result.dispatchCommand).toBe('runtime_v2_projector_test_event');
});

test('external projector bridge bootstraps queries streams and reports', async ({ page }) => {
  await page.goto('about:blank');
  await page.addScriptTag({ path: runtimeClient });

  const result = await page.evaluate(async () => {
    const subscribers = new Set();
    const dispatched = [];
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
    client.close();
    return {
      initialRevision: initial.revision,
      host,
      revisions,
      command: dispatched[0].command,
      remainingSubscribers: subscribers.size,
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
});
