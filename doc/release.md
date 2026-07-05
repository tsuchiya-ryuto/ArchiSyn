# リリース手順

ArchiSyn のバージョニングと配布（Phase 4）の手順をまとめます。

## 前提

- GitHub リポジトリに push 済みであること（Releases・Actions を使用）
- バージョンは [Semantic Versioning](https://semver.org/lang/ja/)（`x.y.z`）

## バージョンの所在

バージョンは次の3ファイルに存在し、`npm run version:bump` で一括更新します。

| ファイル                    | 用途                           |
| --------------------------- | ------------------------------ |
| `package.json`              | フロントエンド                 |
| `src-tauri/tauri.conf.json` | アプリ本体・インストーラの版数 |
| `src-tauri/Cargo.toml`      | Rust クレート                  |

## 手順

```bash
# 1. バージョンを一括更新
npm run version:bump -- 0.2.0

# 2. Cargo.lock を追従させる
cd src-tauri && cargo check && cd ..

# 3. コミットしてタグを push
git add -A && git commit -m "v0.2.0"
git tag v0.2.0
git push origin master --tags
```

タグ（`v*`）の push で `.github/workflows/release.yml` が起動し、以下が自動実行されます:

1. Linux（`.deb` / `.rpm` / `.AppImage`）、macOS（Apple Silicon / Intel の `.dmg`）、
   Windows（`.msi` / NSIS `.exe`）をそれぞれの OS ランナーでビルド
2. GitHub Releases に **ドラフト** として作成し、全インストーラを添付

ドラフトの内容（リリースノート）を確認・編集してから公開してください。

## インストール / アンインストール

| OS            | インストール                                           | アンインストール                                                         |
| ------------- | ------------------------------------------------------ | ------------------------------------------------------------------------ |
| Ubuntu/Debian | `sudo apt install ./ArchiSyn_x.y.z_amd64.deb`          | `sudo apt remove archi-syn`（パッケージ名は `archi-syn`）                |
| Fedora 系     | `sudo dnf install ./ArchiSyn-x.y.z-1.x86_64.rpm`       | `sudo dnf remove archi-syn`                                              |
| Linux（共通） | `.AppImage` に実行権限を付けて起動（インストール不要） | ファイル削除のみ                                                         |
| macOS         | `.dmg` を開いて Applications へドラッグ                | Applications から削除                                                    |
| Windows       | `.msi` / `.exe` を実行                                 | 「設定 > アプリ」から ArchiSyn を削除（MSI/NSIS の標準アンインストーラ） |

## ローカルでのビルド確認（Linux）

```bash
npm run tauri build
# 生成物: src-tauri/target/release/bundle/{deb,rpm,appimage}/
```

> 注意: Tauri はクロスコンパイル非推奨のため、macOS / Windows のインストーラは
> それぞれの OS（CI の各ランナー）でビルドします（doc/setup.md D 参照）。

---

_更新履歴_

- 2026-07-06: 初版作成（Phase 4）
