// README 用デモ動画の録画スクリプト。
// 前提: vite が http://localhost:1420 で起動していること（npm run dev / tauri dev）。
// 使い方: node scripts/record-demo.mjs → scripts/.demo-video/ に webm が出力される。
import { chromium } from "playwright";

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const browser = await chromium.launch();
const context = await browser.newContext({
  viewport: { width: 1280, height: 800 },
  recordVideo: {
    dir: "scripts/.demo-video",
    size: { width: 1280, height: 800 },
  },
});
const page = await context.newPage();
await page.goto("http://localhost:1420");
await sleep(1500);

// --- ノードを3つ追加 ---
for (let i = 0; i < 3; i++) {
  await page.click("text=+ ノード追加");
  await sleep(500);
}

// --- ノードをドラッグで配置 ---
async function moveNode(label, x, y) {
  const header = page
    .locator(`.react-flow__node:has-text("${label}") .arch-node-header`)
    .first();
  const box = await header.boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(x, y, { steps: 15 });
  await page.mouse.up();
  await sleep(300);
}
await moveNode("Node1", 180, 350);
await moveNode("Node2", 480, 300);
await moveNode("Node3", 800, 350);

// --- ラベルを変更 ---
async function rename(oldLabel, newLabel) {
  await page.click(`.react-flow__node:has-text("${oldLabel}") .arch-node-header`);
  await sleep(400);
  const input = page
    .locator('.field:has-text("ラベル")')
    .locator("input")
    .first();
  await input.click();
  await input.fill(newLabel);
  await input.press("Enter");
  await sleep(500);
}
await rename("Node1", "ImuDriver");
await rename("Node2", "SensorFusion");
await rename("Node3", "Controller");

// --- ポート型を検索して設定（ImuDriver の出力 / SensorFusion の入力）---
async function setPortType(nodeLabel, section, query, pick) {
  await page.click(`.react-flow__node:has-text("${nodeLabel}") .arch-node-header`);
  await sleep(400);
  const input = page
    .locator(`section.inspector-section:has(h3:text("${section}")) .type-search input`)
    .first();
  await input.click();
  await input.fill(query);
  await sleep(800); // 検索ドロップダウンを見せる
  await page.click(`.type-search-list li:has-text("${pick}")`);
  await sleep(500);
}
await setPortType("ImuDriver", "出力ポート", "imu", "sensor_msgs/Imu");
await setPortType("SensorFusion", "入力ポート", "imu", "sensor_msgs/Imu");

// --- 接続（型が一致するので繋がる）---
async function connect(fromLabel, toLabel) {
  const src = page
    .locator(`.react-flow__node:has-text("${fromLabel}") .port-row-out .port-handle`)
    .first();
  const dst = page
    .locator(`.react-flow__node:has-text("${toLabel}") .port-row-in .port-handle`)
    .first();
  const a = await src.boundingBox();
  const b = await dst.boundingBox();
  await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
  await page.mouse.down();
  await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, { steps: 20 });
  await sleep(200);
  await page.mouse.up();
  await sleep(600);
}
await connect("ImuDriver", "SensorFusion");
await connect("SensorFusion", "Controller");

// --- 型タブ: 表から貼り付け ---
await page.click('.sidebar-tabs button:has-text("型")');
await sleep(500);
await page.click('button:has-text("表から貼り付け")');
await sleep(600);
await page.fill(
  ".paste-area",
  "FusedPose\tposition\tgeometry_msgs/Vector3\n\tconfidence\tfloat64",
);
await sleep(1500); // プレビューを見せる
await page.click(".dialog-actions .generate-button");
await sleep(800);

// --- Launch タブ: 引数を追加 ---
await page.click('.sidebar-tabs button:has-text("Launch")');
await sleep(500);
await page.click(
  'section.inspector-section:has(h3:text("launch 引数")) button:has-text("+ 追加")',
);
await sleep(400);
const argName = page
  .locator('section.inspector-section:has(h3:text("launch 引数")) .edit-name')
  .first();
await argName.click();
await argName.fill("use_sim_time");
await argName.press("Enter");
await sleep(1200);

// --- 最後にキャンバス全体を見せて終了 ---
await page.click('.sidebar-tabs button:has-text("ノード")');
await sleep(2000);

await context.close();
const video = await page.video().path();
console.log(`video: ${video}`);
await browser.close();
