#!/usr/bin/env node
// バージョンを package.json / src-tauri/tauri.conf.json / src-tauri/Cargo.toml に同期する。
// 使い方: npm run version:bump -- 0.2.0
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) {
  console.error("使い方: npm run version:bump -- <x.y.z>");
  process.exit(1);
}

function update(path, replace) {
  const before = readFileSync(path, "utf8");
  const after = replace(before);
  if (before === after) {
    console.error(`変更なし（パターン不一致の可能性）: ${path}`);
    process.exit(1);
  }
  writeFileSync(path, after);
  console.log(`updated: ${path}`);
}

update("package.json", (t) =>
  t.replace(/("version":\s*")[^"]+(")/, `$1${version}$2`),
);
update("src-tauri/tauri.conf.json", (t) =>
  t.replace(/("version":\s*")[^"]+(")/, `$1${version}$2`),
);
update("src-tauri/Cargo.toml", (t) =>
  t.replace(/^(version\s*=\s*")[^"]+(")/m, `$1${version}$2`),
);

console.log(`\nバージョンを ${version} に更新しました。次の手順:`);
console.log("  1. cd src-tauri && cargo check   # Cargo.lock の更新");
console.log("  2. git commit -am 'vX.Y.Z'");
console.log(`  3. git tag v${version} && git push origin master --tags`);
