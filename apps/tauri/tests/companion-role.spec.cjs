const { test, expect } = require('@playwright/test');
const { pathToFileURL } = require('node:url');
const path = require('node:path');

const uiUrl = pathToFileURL(path.resolve(__dirname, '../ui/index.html')).href + '?role=control';

test('companion discovery requires fingerprint confirmation before pairing', async ({ page }) => {
  await page.addInitScript(() => {
    const publicState = {
      app_role: 'companion_projector',
      runtime_instance_id: 'companion-dormant-runtime',
      revision: 0,
      counter: 0,
      external_display_count: 0,
      board: { enabled: false, phase: 'disabled' },
      game: null,
      projector_output: 'external_display',
      companion_port: null,
      companion_available: false
    };
    window.__nativeCalls = [];
    window.__nativeEvents = {};
    window.__TAURI__ = {
      core: {
        invoke: async (command, args = {}) => {
          window.__nativeCalls.push({ command, args });
          switch (command) {
            case 'runtime_bootstrap':
              return publicState;
            case 'companion_discovery_start':
            case 'companion_discovery_stop':
              return null;
            case 'companion_client_status':
              return null;
            case 'companion_discovered_hosts':
              return [{
                service_name: 'Smart Dartboard Arcade',
                host_name: 'arcade-mac.local',
                port: 43123,
                host_id: '991708fa-c4e7-419f-ad1d-c44f01891b03',
                protocol_version: 1,
                tls: true
              }];
            case 'companion_pairing_prepare':
              return {
                host_id: args.hostId,
                service_name: 'Smart Dartboard Arcade',
                manual_fingerprint: 'A1B2-C3D4-E5F6-0718',
                expires_at_ms: Date.now() + 300000
              };
            case 'companion_pairing_complete':
              return {
                host_id: args.hostId,
                service_name: 'Smart Dartboard Arcade',
                paired: true,
                phase: 'discovering',
                runtime_instance_id: null,
                revision: null
              };
            default:
              throw new Error(`unexpected native command: ${command}`);
          }
        }
      },
      event: {
        listen: async (name, listener) => {
          window.__nativeEvents[name] = listener;
          return () => delete window.__nativeEvents[name];
        }
      }
    };
  });

  await page.goto(uiUrl);
  await expect(page.getByRole('heading', { name: /Controller finden/ })).toBeVisible();
  await expect(page.getByText('Smart Dartboard Arcade')).toBeVisible();
  await page.getByRole('button', { name: 'Auswählen' }).click();

  await expect(page.locator('#clientPairingFingerprint')).toHaveText('A1B2-C3D4-E5F6-0718');
  const complete = page.getByRole('button', { name: 'Sicher koppeln' });
  await expect(complete).toBeDisabled();
  await page.locator('#clientPairingCode').fill('123456');
  await expect(complete).toBeDisabled();
  await page.locator('#fingerprintConfirmed').check();
  await expect(complete).toBeEnabled();
  await complete.click();

  await expect(page.locator('#clientPairingStatus')).toContainText('gekoppelt');
  const completion = await page.evaluate(() =>
    window.__nativeCalls.find(call => call.command === 'companion_pairing_complete'));
  expect(completion.args).toEqual({
    hostId: '991708fa-c4e7-419f-ad1d-c44f01891b03',
    manualFingerprint: 'A1B2-C3D4-E5F6-0718',
    code: '123456'
  });
  expect(await page.evaluate(() =>
    window.__nativeCalls.some(call => call.command === 'runtime_dispatch'))).toBe(false);

  await page.evaluate(() => {
    window.__nativeEvents['companion-projector-status']({ payload: {
      host_id: '991708fa-c4e7-419f-ad1d-c44f01891b03',
      service_name: 'Smart Dartboard Arcade',
      paired: true,
      phase: 'connected',
      runtime_instance_id: 'controller-runtime',
      revision: 8
    }});
    window.__nativeEvents['companion-projector-frame']({ payload: {
      runtime_instance_id: 'controller-runtime',
      revision: 8,
      counter: 120
    }});
  });
  await expect(page.locator('#companionProjectorStage')).toBeVisible();
  await expect(page.locator('#companionCounter')).toHaveText('120');
  await expect(page.locator('#companionRuntimeStatus')).toContainText('Revision 8');

  await page.evaluate(() => {
    window.__nativeEvents['companion-projector-status']({ payload: {
      host_id: '991708fa-c4e7-419f-ad1d-c44f01891b03',
      service_name: 'Smart Dartboard Arcade',
      paired: true,
      phase: 'reconnecting',
      runtime_instance_id: null,
      revision: null
    }});
  });
  await expect(page.locator('#companionProjectorStage')).toBeHidden();
  await expect(page.locator('#discoveryStatus')).toContainText('neuer Snapshot');
});
