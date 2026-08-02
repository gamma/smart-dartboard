const { test, expect } = require('@playwright/test');
const { pathToFileURL } = require('node:url');
const path = require('node:path');

const uiUrl = pathToFileURL(path.resolve(__dirname, '../../../web/native.html')).href + '?role=control';

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
      companion_available: false,
      companion_protocol_version: 2
    };
    const productEnvelope = {
      protocol_version: 1,
      kind: 'state',
      message_id: 'companion-product',
      runtime_instance_id: 'controller-runtime',
      revision: 8,
      payload: {
        revision: 8,
        session: {
          session_id: 'session-1',session_status: 'active',screen: 'playing',
          game_id: 'game-1',players: [{id:'ada',name:'Ada',avatar:'fox',color:'#ff00aa'}],
          standings: [{player_id:'ada',games:0,wins:0,session_points:0}],
        },
        game: {game_type:'count_up',state:{
          players:[{id:'ada',name:'Ada',score:120}],current_player_index:0,
          round_number:2,darts_in_turn:1,turn_score:60,status:'running',last_event:null,
        }},
        settings: {},
      },
    };
    const modes=[{
      slug:'countup',title:'Count Up',tagline:'Jeder Punkt zählt',description:'Punkte sammeln',
      accent:'#28e7ff',accent_secondary:'#176dff',visual:'neon-orbit',icon:'target',
      artwork:'/static/assets/modes/countup.webp',sound_theme:'arena',min_players:1,max_players:8,
      ruleset_version:1,options:[],instructions:[],control_legend:[],
    }];
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
            case 'companion_projector_v2_bootstrap':
              return productEnvelope;
            case 'companion_projector_v2_query':
              if(args.path==='/api/v2/modes') return modes;
              if(args.path==='/api/v2/players' || args.path==='/api/v2/statistics/players') return [];
              if(args.path==='/api/v2/host') return {
                app_role:'companion_projector',test_events:false,
                board:{enabled:false,phase:'disabled'},
              };
              throw new Error(`unexpected Companion query: ${args.path}`);
            case 'companion_projector_v2_report':
              if(!['report_projector_geometry','report_sound_status'].includes(args.envelope.command.type)){
                throw new Error(`forbidden Companion report: ${args.envelope.command.type}`);
              }
              return {command_id:args.envelope.command_id,revision:8,duplicate:false};
            case 'companion_discovered_hosts':
              return [{
                service_name: 'Smart Dartboard Arcade',
                host_name: 'arcade-mac.local',
                port: 43123,
                host_id: '991708fa-c4e7-419f-ad1d-c44f01891b03',
                protocol_version: 2,
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
  });
  await expect(page).toHaveURL(/projector\.html\?native-companion=1/);
  await expect(page.locator('.projection-game')).toBeVisible();
  await expect(page.locator('[data-score-player="ada"]').first()).toHaveText('120');
  await expect(page.locator('.projector-test-tools')).toHaveCount(0);
  await expect.poll(async()=>page.evaluate(() => window.__nativeCalls
    .some(call=>call.command==='companion_projector_v2_report'
      && call.args.envelope.command.type==='report_projector_geometry'))).toBe(true);
  expect(await page.evaluate(() => window.__nativeCalls
    .some(call=>call.command==='runtime_v2_dispatch'))).toBe(false);

  await page.evaluate(() => {
    window.__nativeEvents['companion-projector-v2-disconnected']({payload:null});
  });
  await expect(page).toHaveURL(/native\.html\?role=control/);
  await expect(page.getByRole('heading', { name: /Controller finden/ })).toBeVisible();
});
