# ArchiSyn 開発環境セットアップ

`doc/plan.md` の技術スタックに基づく開発環境構築手順をまとめます。

ソフトウェアは目的別に **A. ArchiSyn 本体の開発**（必須）と **B. 生成コードの動作検証**（推奨）に分けて記載します。

---

## 0. 前提（現在の環境）

2026-05-20 時点で開発機 (`tsuchiya-ryuto@Ubuntu 24.04.4 LTS`) にインストール済み：

| 項目                               | バージョン                             | 状態 |
| ---------------------------------- | -------------------------------------- | ---- |
| OS                                 | Ubuntu 24.04.4 LTS (Noble Numbat)      | —    |
| Rust / Cargo                       | 1.95.0                                 | ✅   |
| nvm                                | 0.40.1                                 | ✅   |
| Node.js                            | v24.15.0 (lts/krypton)                 | ✅   |
| npm                                | 11.12.1                                | ✅   |
| `cargo-create-tauri-app`           | latest                                 | ✅   |
| Tauri apt 依存パッケージ一式 (A.2) | —                                      | ✅   |
| Git                                | 2.43.0                                 | ✅   |
| Docker                             | 29.4.2（`docker` グループ参加済み）    | ✅   |
| g++ / cmake                        | 13.3.0 / 3.28.3                        | ✅   |
| Python                             | 3.12.3                                 | ✅   |
| VS Code                            | 1.118.1                                | ✅   |
| ROS 補助 apt パッケージ            | `ros-build-essential`, `ros-dev-tools` | ✅   |

> **重要**: ROS 2 Humble は公式には **Ubuntu 22.04 (Jammy)** 対応です。24.04 へのネイティブインストールは非サポートのため、本ドキュメントでは **Docker での Humble 利用** を推奨します（後述 B.1）。
> 将来的に Jazzy へ追従する場合は `MiddlewareAdapter` の差し替えで対応可能です（plan.md Phase 3）。

> 以降の各セクションは「新規マシン / チームメンバ向けの再現手順」として残しています。既にインストール済みの項目には ✅ を付記しています。

---

## A. ArchiSyn 本体の開発（必須）

### A.1 Rust toolchain ✅ 1.95.0

```bash
# 既にインストール済みの場合
rustup update stable
rustup default stable

# 未インストールの場合
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

確認：

```bash
rustc --version   # rustc 1.9x.x
cargo --version
```

### A.2 Tauri のシステム依存パッケージ（Linux: Ubuntu 24.04） ✅ 導入済

Tauri v2 は WebKitGTK 4.1 を使用します。

```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  pkg-config
```

> 参考: https://tauri.app/start/prerequisites/

### A.3 Node.js（推奨: 20 LTS 以上 / nvm 経由） ✅ nvm 0.40.1 / Node v24.15.0 (lts/krypton)

`apt` 経由ではバージョンが古い場合があるため、`nvm` 推奨です。

```bash
# nvm インストール
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
# シェル再起動 or:
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

# Node.js LTS をインストール
nvm install --lts
nvm use --lts
```

確認：

```bash
node --version   # v20.x 以上（現環境: v24.15.0）
npm --version
```

> 新しいシェルで `node` が見つからない場合、`~/.bashrc` に nvm の読み込みが追記されているか確認してください。

### A.4 Tauri CLI ✅ `cargo-create-tauri-app` 導入済

プロジェクト初期化時に `cargo create-tauri-app` を使うため、入れておくと便利。

```bash
cargo install create-tauri-app --locked
# プロジェクト作成後はリポジトリの devDependencies (@tauri-apps/cli) を使うので
# グローバル @tauri-apps/cli は不要
```

### A.5 Git ✅ 2.43.0

```bash
git --version
# 無ければ: sudo apt install -y git
```

### A.6 動作確認 ✅ 2026-07-04 実施済（ビルド成功・ウィンドウ起動確認）

```bash
cd /tmp
cargo create-tauri-app hello-tauri --template react-ts --manager npm --yes
cd hello-tauri
npm install
npm run tauri dev   # ウィンドウが開けば OK
```

---

## B. 生成コードの動作検証（推奨）

ArchiSyn 本体の開発のみなら不要ですが、生成された ROS 2 ワークスペースのビルド・実行確認には必要です。

### B.1 ROS 2 Humble（Docker 推奨） ✅ Docker 29.4.2 / グループ参加済

Ubuntu 24.04 ではネイティブインストール非対応のため、公式イメージを使います。

```bash
# Docker 未導入の場合
sudo apt install -y docker.io
sudo usermod -aG docker $USER
# 再ログイン後、グループ反映を確認: groups | grep docker

# Humble + 開発ツール入りイメージ（未取得の場合のみ）
docker pull osrf/ros:humble-desktop-full
```

> 補助 apt パッケージとして `ros-build-essential`, `ros-dev-tools` がホスト側にも導入済みです（ホストから `colcon` を直接使うケース向け）。

動作確認（コンテナ内で talker/listener）：

```bash
docker run --rm -it osrf/ros:humble-desktop-full bash -c \
  "source /opt/ros/humble/setup.bash && ros2 run demo_nodes_cpp talker"
```

ArchiSyn が生成したワークスペースをホストからマウントしてビルド：

```bash
docker run --rm -it \
  -v $(pwd)/generated_ws:/ws \
  -w /ws \
  osrf/ros:humble-desktop-full \
  bash -c "source /opt/ros/humble/setup.bash && colcon build"
```

> `colcon`, `rosdep` はイメージに同梱されています。

### B.2 言語別ツール

#### Python（Phase 1） ✅ Python 3.12.3

ROS 2 イメージに `python3` と `rclpy` は同梱されているため追加は不要。ローカルで補助的に動かす場合：

```bash
sudo apt install -y python3 python3-pip python3-venv
```

#### C++（Phase 2） ✅ g++ 13.3.0 / cmake 3.28.3

ローカルでのビルド補助：

```bash
sudo apt install -y g++ cmake
```

ROS 2 イメージ内では `rclcpp`, `colcon` で完結。

#### Rust（Phase 2）

`ros2_rust` / `r2r` どちらを採用するかは Phase 2 評価時に決定。
A.1 の Rust toolchain で足りますが、必要に応じて以下：

```bash
rustup component add rust-src rustfmt clippy
```

---

## C. 推奨エディタ・補助ツール ✅ VS Code 1.118.1 導入済

| ツール                          | 用途                            |
| ------------------------------- | ------------------------------- |
| VS Code                         | フロント / Rust 共通            |
| 拡張: `rust-lang.rust-analyzer` | Rust LSP                        |
| 拡張: `tauri-apps.tauri-vscode` | Tauri 設定補助                  |
| 拡張: `dbaeumer.vscode-eslint`  | ESLint                          |
| 拡張: `esbenp.prettier-vscode`  | Prettier                        |
| 拡張: `redhat.vscode-yaml`      | `.arcsyn` (YAML) のスキーマ検証 |

```bash
# Ubuntu に snap で
sudo snap install code --classic
```

> 拡張のインストール状態は VS Code 起動後に手動確認 / 導入してください。

---

## D. クロスプラットフォーム配布時の参考（Phase 4 用）

開発機（Linux）以外のビルドは各 OS 上で行う必要があります（Tauri はクロスコンパイル非推奨）。

| OS      | 必要なもの                                                         |
| ------- | ------------------------------------------------------------------ |
| macOS   | Xcode Command Line Tools (`xcode-select --install`), Rust, Node.js |
| Windows | Microsoft C++ Build Tools, WebView2 (Win11 標準), Rust, Node.js    |

詳細は Phase 4 着手時に追記。

---

## E. セットアップ チェックリスト

ArchiSyn 本体の開発を始めるための最小チェックリスト（現開発機の状態）：

- [x] `rustc --version` → 1.8x 以上（現: 1.95.0）
- [x] Tauri 用 apt パッケージ（A.2）導入済
- [x] `node --version` → v20 以上（現: v24.15.0）
- [x] `cargo create-tauri-app` 実行可能
- [x] A.6 の hello-tauri が起動できる（2026-07-04 確認済）

セットアップは完了。plan.md の **Phase 0** に着手できます。

---

_更新履歴_

- 2026-05-20: 初版作成
- 2026-05-20: 開発機への各種ツール導入を反映（Rust 1.95.0 / Node v24.15.0 / Docker 29.4.2 / VS Code 1.118.1 など）
- 2026-07-04: A.6 動作確認を実施（hello-tauri ビルド成功・起動確認）。チェックリスト完了
