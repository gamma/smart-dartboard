const { test, expect } = require('@playwright/test');
const path = require('node:path');

const runtimeClient = path.resolve(__dirname, '../../../web/static/runtime-client.js');

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
