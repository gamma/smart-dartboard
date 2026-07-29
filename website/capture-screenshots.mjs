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

async function capture(page, filename) {
  await page.screenshot({
    path: resolve(output, filename),
    type: "jpeg",
    quality: 88,
    animations: "disabled",
  });
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
} finally {
  await browser.close();
  await api.dispose();
}

console.log(`Captured website screenshots in ${output}`);
