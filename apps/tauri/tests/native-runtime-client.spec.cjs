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
