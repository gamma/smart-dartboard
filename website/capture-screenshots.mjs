import { mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const playwrightModule =
  process.env.PLAYWRIGHT_MODULE ||
  "/opt/homebrew/lib/node_modules/playwright/index.mjs";
const { request, webkit } = await import(playwrightModule);

const baseURL = process.env.SDB_CAPTURE_URL || "http://127.0.0.1:8777";
const here = dirname(fileURLToPath(import.meta.url));
const output = resolve(here, "assets/screenshots");
await mkdir(output, { recursive: true });

const api = await request.newContext({ baseURL });

async function post(path, data) {
  const response = await api.post(path, data === undefined ? {} : { data });
  if (!response.ok()) {
    throw new Error(`${path}: ${response.status()} ${await response.text()}`);
  }
  return response.json();
}

async function openPage(browser, path, viewport) {
  const context = await browser.newContext({
    baseURL,
    viewport,
    deviceScaleFactor: 1,
    colorScheme: "dark",
  });
  const page = await context.newPage();
  await page.goto(path, { waitUntil: "domcontentloaded" });
  await page.waitForTimeout(900);
  return { context, page };
}

async function capture(page, filename, animations = "disabled") {
  await page.screenshot({
    path: resolve(output, filename),
    type: "jpeg",
    quality: 88,
    animations,
  });
}

function throwEvent(target, seq) {
  return {
    type: "hit",
    seq,
    field: target.field,
    ring: target.ring,
    score: target.score,
    multiplier: target.multiplier,
    label: target.label,
  };
}

async function playTurn(darts, seqStart) {
  let state;
  for (let index = 0; index < darts.length; index += 1) {
    state = await post("/api/throw/manual", {
      type: "hit",
      seq: seqStart + index,
      ...darts[index],
    });
  }
  if (state.status !== "finished") {
    await post("/api/continue");
  }
}

const ada = await post("/api/players", {
  name: "Ada",
  avatar: "comet",
  color: "#28e7ff",
});
const bob = await post("/api/players", {
  name: "Bob",
  avatar: "fox",
  color: "#ff795f",
});
const mia = await post("/api/players", {
  name: "Mia",
  avatar: "star",
  color: "#f4d35e",
});
await post("/api/session/start", {
  player_ids: [ada.id, bob.id, mia.id],
});

const browser = await webkit.launch();

try {
  {
    const { context, page } = await openPage(
      browser,
      "/control",
      { width: 1440, height: 1000 },
    );
    await capture(page, "control-mode-selection.jpg");
    await context.close();
  }

  await post("/api/game/prepare", {
    game_type: "candy_cannon",
    options: { rounds: 5 },
  });
  await post("/api/game/start");
  await post("/api/game/live");
  await post("/api/event", {
    type: "hit",
    seq: 1,
    field: 25,
    ring: "single_bull",
  });
  await post("/api/event", {
    type: "hit",
    seq: 2,
    field: 25,
    ring: "single_bull",
  });

  {
    const { context, page } = await openPage(
      browser,
      "/projector",
      { width: 1600, height: 900 },
    );
    await page.addStyleTag({
      content: ".projector-test-tools{display:none!important}",
    });
    await capture(page, "projector-candy-cannon.jpg");
    await context.close();
  }

  {
    const { context, page } = await openPage(
      browser,
      "/control",
      { width: 1280, height: 900 },
    );
    await capture(page, "control-candy-cannon.jpg");
    await context.close();
  }

  await post("/api/game/abort");
  await post("/api/game/prepare", {
    game_type: "block_drop",
    options: { drop_flow: "continue" },
  });
  await post("/api/game/start");
  await post("/api/game/live");
  await post("/api/event", {
    type: "hit",
    seq: 11,
    field: 12,
    ring: "single_outer",
  });
  await post("/api/event", {
    type: "hit",
    seq: 12,
    field: 4,
    ring: "single_outer",
  });

  {
    const { context, page } = await openPage(
      browser,
      "/projector",
      { width: 1600, height: 900 },
    );
    await page.addStyleTag({
      content: ".projector-test-tools{display:none!important}",
    });
    await capture(page, "projector-block-drop.jpg");
    await context.close();
  }

  await post("/api/game/abort");
  await post("/api/game/prepare", {
    game_type: "avoid_bomb",
    options: {
      rounds: 5,
      bomb_count: 6,
      bomb_growth: "escalating",
      penalty: -50,
    },
  });
  await post("/api/game/start");
  await post("/api/game/live");

  {
    const { context, page } = await openPage(
      browser,
      "/projector",
      { width: 1920, height: 1080 },
    );
    await page.addStyleTag({
      content: ".projector-test-tools{display:none!important}",
    });
    await capture(page, "projector-avoid-bomb.jpg");
    await context.close();
  }

  await post("/api/event", { type: "miss", seq: 20 });
  await post("/api/game/abort");
  await post("/api/game/prepare", {
    game_type: "dragon_eggs",
    options: { rounds: 5, eggs: 4 },
  });
  await post("/api/game/start");
  const dragonState = await post("/api/game/live");
  const dragonScale = dragonState.game.overlay.danger[0];
  await post("/api/event", throwEvent(dragonScale, 21));
  await post("/api/event", throwEvent(dragonScale, 22));

  {
    const { context, page } = await openPage(
      browser,
      "/projector",
      { width: 1600, height: 900 },
    );
    await page.addStyleTag({
      content: ".projector-test-tools{display:none!important}",
    });
    await post("/api/event", throwEvent(dragonScale, 23));
    await page.waitForTimeout(250);
    await capture(page, "projector-dragon-eggs.jpg", "allow");
    await context.close();
  }

  await post("/api/game/abort");
  await post("/api/game/prepare", {
    game_type: "countup",
    options: { rounds: 5 },
  });
  await post("/api/game/start");
  await post("/api/game/live");

  const countupTurns = [
    [
      { field: 20, ring: "triple", score: 60, multiplier: 3, label: "T20" },
      { field: 20, ring: "single_outer", score: 20, multiplier: 1, label: "S20" },
      { field: 19, ring: "triple", score: 57, multiplier: 3, label: "T19" },
    ],
    [
      { field: 18, ring: "triple", score: 54, multiplier: 3, label: "T18" },
      { field: 20, ring: "single_inner", score: 20, multiplier: 1, label: "S20" },
      { field: 5, ring: "single_outer", score: 5, multiplier: 1, label: "S5" },
    ],
    [
      { field: 17, ring: "double", score: 34, multiplier: 2, label: "D17" },
      { field: 19, ring: "single_inner", score: 19, multiplier: 1, label: "S19" },
      { field: 25, ring: "single_bull", score: 25, multiplier: 1, label: "SBull" },
    ],
  ];
  let countupSeq = 100;
  for (let round = 0; round < 5; round += 1) {
    for (const turn of countupTurns) {
      await playTurn(turn, countupSeq);
      countupSeq += 3;
    }
  }
  await post("/api/game/next");

  {
    const { context, page } = await openPage(
      browser,
      "/control",
      { width: 1440, height: 1000 },
    );
    await page.locator('[data-action="open-history"]').click();
    await page.locator(".history-dashboard").waitFor();
    await capture(page, "control-statistics.jpg");

    const sessions = await (await api.get("/api/history/sessions?limit=10")).json();
    const sessionId = sessions.sessions[0].id;
    const session = await (
      await api.get(`/api/history/sessions/${encodeURIComponent(sessionId)}`)
    ).json();
    const finishedGame = session.games.find(
      game => game.game_type === "countup" && game.status === "finished",
    );
    await page.locator(
      `[data-action="history-session"][data-id="${sessionId}"]`,
    ).click();
    await page.locator(
      `[data-action="history-game"][data-id="${finishedGame.id}"]`,
    ).click();
    await page.locator(".replay-view").waitFor();
    await page.evaluate(() => window.scrollTo(0, 0));

    const replay = await (
      await api.get(`/api/history/games/${encodeURIComponent(finishedGame.id)}/replay`)
    ).json();
    const replayIndex = replay.events.findIndex(
      event => event.event_type === "throw" && event.payload?.label === "T20",
    );
    if (replayIndex >= 0) {
      await page.locator("#replayRange").evaluate((element, value) => {
        element.value = String(value);
        element.dispatchEvent(new Event("input", { bubbles: true }));
      }, replayIndex);
    }
    await page.waitForTimeout(100);
    await capture(page, "control-replay.jpg");
    await context.close();
  }
} finally {
  await browser.close();
  await api.dispose();
}

console.log(`Captured website screenshots in ${output}`);
