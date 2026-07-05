# ArchiSyn 実装計画 (plan.md)

本ドキュメントは `doc/required_spec.md` を入力として、ArchiSyn の実装計画をまとめたものです。
ユーザーとの合意済み技術選定に基づき、MVP からのフェーズ分けで進めます。

---

## 1. 技術選定（合意済み）

| 項目               | 採用技術                                                         | 理由                                                                         |
| ------------------ | ---------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| デスクトップアプリ | **Tauri (Rust + Web)**                                           | 軽量・高速。Rustでコード生成ロジックを書けばサポート言語の1つ（Rust）と相性◎ |
| ノードグラフUI     | **React Flow (xyflow)**                                          | ノードベースエディタのデファクト。カスタマイズ性が高い                       |
| フロントエンド     | **React + TypeScript + Vite**                                    | React Flow との親和性                                                        |
| ファイル形式       | **YAML** (`.arcsyn`)                                             | テキスト・Git親和・コメント可。GUI完全復元に必要な全状態を含める             |
| コード生成         | **Rust (Tauriバックエンド側)** + **テンプレートエンジン (Tera)** | Tauriの`#[tauri::command]`から呼び出し可能                                   |
| ミドルウェア       | **ROS 2 Humble**                                                 | 抽象化レイヤ経由（Phase 3 で本格対応）                                       |
| 対象OS             | Linux / macOS / Windows                                          | Tauri が標準対応                                                             |

---

## 2. 全体アーキテクチャ

```
┌─────────────────────────────────────────────────────┐
│                  ArchiSyn Desktop                    │
│ ┌─────────────────────────────────────────────────┐ │
│ │ Frontend (React + TypeScript)                    │ │
│ │  ├─ React Flow Canvas (ノード/エッジ編集)         │ │
│ │  ├─ Type Editor (カスタム型管理)                  │ │
│ │  ├─ Node Inspector (周期/言語/パラメータ)         │ │
│ │  └─ File Menu (新規/開く/保存/コード生成)         │ │
│ └────────────────────┬────────────────────────────┘ │
│                      │ Tauri IPC (#[command])       │
│ ┌────────────────────┴────────────────────────────┐ │
│ │ Backend (Rust)                                   │ │
│ │  ├─ .arcsyn パーサ/シリアライザ (serde_yaml)      │ │
│ │  ├─ コード生成エンジン                            │ │
│ │  │   ├─ MiddlewareAdapter trait (抽象化)          │ │
│ │  │   │   └─ Ros2HumbleAdapter (Phase 1)           │ │
│ │  │   ├─ LanguageGenerator trait                   │ │
│ │  │   │   ├─ PythonGenerator (Phase 1)             │ │
│ │  │   │   ├─ CppGenerator    (Phase 2)             │ │
│ │  │   │   └─ RustGenerator   (Phase 2)             │ │
│ │  │   └─ Tera テンプレート群                       │ │
│ │  └─ ファイル I/O (既存実装ファイルの保護)         │ │
│ └─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

---

## 3. ディレクトリ構造（リポジトリ）

```
ArchiSyn/
├── doc/
│   ├── plan.md                   ← 本ファイル
│   ├── required_spec.md
│   └── setup.md                  ← 開発環境セットアップ手順
├── README.md
├── src-tauri/                    ← Tauri Rust バックエンド
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs
│       ├── commands.rs           ← IPC エントリポイント
│       ├── model/                ← .arcsyn データモデル
│       │   ├── mod.rs
│       │   ├── project.rs
│       │   ├── node.rs
│       │   ├── edge.rs
│       │   └── type_def.rs
│       ├── codegen/
│       │   ├── mod.rs
│       │   ├── middleware/
│       │   │   ├── mod.rs        ← MiddlewareAdapter trait
│       │   │   └── ros2_humble.rs
│       │   ├── language/
│       │   │   ├── mod.rs        ← LanguageGenerator trait
│       │   │   ├── python.rs
│       │   │   ├── cpp.rs
│       │   │   └── rust.rs
│       │   └── templates/        ← Tera テンプレート (.tera)
│       └── fs/
│           └── safe_write.rs     ← 既存ファイル保護 (※1)
├── src/                          ← React + TypeScript フロントエンド
│   ├── App.tsx
│   ├── main.tsx
│   ├── components/
│   │   ├── Canvas/               ← React Flow ラッパ
│   │   ├── NodeInspector/
│   │   ├── TypeEditor/
│   │   └── Menu/
│   ├── state/                    ← Zustand 等の状態管理
│   ├── types/                    ← .arcsyn TS 型
│   └── ipc/                      ← Tauri invoke ラッパ
├── tests/                        ← 統合テスト
└── examples/                     ← サンプル .arcsyn
```

---

## 4. `.arcsyn` データモデル（YAML スキーマ案）

GUI 完全復元 (FM-2) に必要な全状態を含めます。

```yaml
arcsyn_version: "0.1"
project:
  name: my_robot
  middleware: ros2_humble # 将来切替可

custom_types: # F-3
  - name: FusedPose
    fields:
      - { name: position, type: geometry_msgs/Vector3 }
      - { name: confidence, type: float64 }

nodes: # F-1, F-2, F-4, F-6
  - id: n_sensor_fusion
    label: SensorFusion
    language: cpp # python | cpp | rust
    period_ms: 50 # 実行周期
    position: { x: 120, y: 200 } # GUI 復元用
    size: { w: 180, h: 80 }
    inputs:
      - { name: imu, type: sensor_msgs/Imu }
      - { name: lidar, type: sensor_msgs/LaserScan }
    outputs:
      - { name: fused, type: FusedPose }
    params:
      - { name: alpha, type: float64, default: 0.7 }

edges: # F-2
  - id: e1
    source: { node: n_sensor_fusion, port: fused }
    target: { node: n_controller, port: pose }

viewport: # GUI 完全復元
  zoom: 1.0
  pan: { x: 0, y: 0 }
```

---

## 5. フェーズ別実装計画

### Phase 0: プロジェクト初期化（1〜2日）

> **前提**: `doc/setup.md` のチェックリスト（E）を完了していること。特に A.6 の hello-tauri 起動確認まで済ませてから着手する。

- [x] `cargo create-tauri-app` で Tauri + React + TypeScript 雛形生成
- [x] React Flow 導入・最小ノード表示確認
- [x] CI（GitHub Actions: build & test）
- [x] `.gitignore`, lint/format (rustfmt, eslint, prettier)
- [x] **マイルストーン**: `npm run tauri dev` で空キャンバスが起動

**Phase 0 完了（2026-07-04）**

### Phase 1: MVP — Python + ROS 2 Humble（2〜3週）

最小の「設計 → ビルド可能なROS 2パッケージ生成」フローを通すフェーズ。

#### 1.1 モデリング基盤

- [x] React Flow にノード追加・削除・接続・移動
- [x] ノードのポート（入出力）UI
- [x] サイドパネル: ノードのプロパティ編集（label, period_ms, params）
- [x] カスタム型エディタ（最小：フィールド名・型のみ）
- [x] 型互換チェック（エッジ接続時のバリデーション）

**1.1 完了（2026-07-04）**

#### 1.2 .arcsyn ファイル I/O

- [x] Rust 側に `Project` 構造体 + `serde_yaml`
- [x] Tauri command: `save_project`, `load_project`, `new_project`
- [x] フロントとの状態同期（Zustand）
- [x] **検証**: 保存→終了→起動→読込で完全復元できる（2026-07-04 GUI 手動確認済）

**1.2 完了（2026-07-04）**

#### 1.3 Python + ROS 2 コード生成

要求仕様の「出力イメージ」に従い、言語別パッケージ構成（`<project>_py_nodes` / `<project>_msgs`）で生成する。
F-5（実行処理部とインターフェース部の分離）は「インターフェース部＝毎回再生成、実装部スケルトン＝既存時は保護（※1）」で実現する。

- [x] `LanguageGenerator` / `MiddlewareAdapter` trait 定義
- [x] `PythonGenerator` + `Ros2HumbleAdapter` 実装
- [x] Tera テンプレート（`<project>_py_nodes` パッケージ、ノード完結型ディレクトリ構成）:
  - `package.xml`, `setup.py`, `setup.cfg`
  - `<project>_py_nodes/<node>/interfaces.py`（インターフェース部。毎回再生成）
  - `<project>_py_nodes/<node>/<node>.py`（実装部スケルトン。rclpy Node, subscribe/publish, timer）
- [x] カスタム型 → `<project>_msgs/msg/*.msg` 生成（共通パッケージ）
- [x] `launch/system.launch.py` 生成
- [x] **実装ファイル保護（※1）**: 実装部の既存ファイルは上書きしない（F-5）
- [x] **検証**: 生成 → `colcon build` 成功 → ノード起動確認
  - 開発機は Ubuntu 24.04 のため ROS 2 Humble はネイティブ非対応。検証は `osrf/ros:humble-desktop-full` コンテナ内で実施（`doc/setup.md` B.1 参照）
  - 2026-07-04 検証済: colcon build 成功 / launch で 2 ノード起動 / エッジ由来の
    トピック配線（Publisher・Subscription 各1）/ パラメータ反映 / 実装部への
    ユーザーコード記入 → 再生成で保護 → カスタム型 FusedPose の Pub/Sub 疎通まで確認

**1.3 完了（2026-07-04）**

#### 1.4 ドキュメント・サンプル

- [x] サンプル `.arcsyn`（3ノード: `examples/demo_robot.arcsyn`。読込・生成可能なことを回帰テストで担保）
- [x] README にクイックスタート追記

**1.4 完了（2026-07-05）**

**Phase 1 完了条件**: GUI で 2 ノードを Pub/Sub 接続し、生成された ROS 2 ワークスペースがビルド・実行できる。

**✅ Phase 1 完了（2026-07-05）** — 2ノード Pub/Sub 構成の生成 → Docker で colcon build → launch 起動 → カスタム型メッセージの疎通まで検証済み。

---

### Phase 2: 多言語対応（2週）

- [x] `CppGenerator` + テンプレート（`rclcpp`, CMakeLists.txt。ノード完結型: `src/<node>/interfaces.hpp` + `<node>.cpp`）
- [x] `RustGenerator` + テンプレート（`ros2_rust` / rclrs 0.7 を採用。ビルドには underlay が必要 → `docker/humble-rust.Dockerfile`）
- [x] ノード単位の言語切替 UI（F-6。Phase 1.1 のインスペクタで実装済）
- [x] 異言語ノード間の型整合（`<project>_msgs` 共通パッケージで担保。混在疎通で確認）
- [x] **検証**: C++/Python 混在ワークスペースで Pub/Sub 動作（2026-07-05 Docker で確認: 3パッケージ colcon build → Python 発行のカスタム型を C++ が受信。実装部保護も両言語で確認）
- [x] **検証（3言語）**: Rust → Python → C++ のチェーンで Pub/Sub 動作（2026-07-05 `archisyn-humble-rust` イメージで確認: 4パッケージ colcon build → Imu(Rust発行) を受けた Python がカスタム型を発行し C++ が受信。実装部保護は3言語で確認）

**Phase 2 完了（2026-07-05）**

---

### Phase 3: ミドルウェア抽象化の本格化（1〜2週）

- [x] `MiddlewareAdapter` インタフェースの再点検
  - ワークスペース生成を `generate()` としてアダプタに完全委譲（対応言語・パッケージ構成・配線はアダプタが決める）
  - 型解決は `RosTypeResolver` に分離し、ROS 系アダプタ間で言語ジェネレータを再利用可能に
- [x] プロジェクトレベルの `middleware:` フィールドで切替（`adapter_for` レジストリ。未対応はエラー。GUI にセレクタ追加）
- [x] ドキュメント: 新しいミドルウェアの追加手順（`doc/adding_middleware.md`）
- [x] スタブの第2アダプタ `mock_pubsub`（ROS 不要・1プロセス Python + インメモリバス。`python3 run.py` で3ノードチェーンの疎通を検証済み）

**Phase 3 完了（2026-07-05）**

---

### Phase 4: 配布・運用（1週）

- [ ] 各 OS 向けインストーラ（Tauri bundle: `.deb` / `.dmg` / `.msi`）
- [ ] アンインストーラ動作確認
- [ ] バージョニング・リリースワークフロー（GitHub Releases）
- [ ] ユーザーマニュアル

---

## 6. リスクと対応

| リスク                                                      | 対応                                                                                           |
| ----------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| React Flow のカスタムノードで Simulink 風 UX を出し切れない | Phase 1 で許容範囲を確認、不足ならカスタムレンダラで補強                                       |
| ros2_rust の安定性                                          | ✅ 解消（2026-07-05）: rclrs 0.7 を採用し Docker 検証済。メッセージ crate は underlay から解決 |
| 既存実装ファイルの保護で edge case（リネーム検知等）        | Phase 1 では「ファイル名一致なら保護」のシンプル方針。Phase 3 でメタデータ管理を検討           |
| YAML の差分が大きくなる                                     | キー順序を安定化（serde_yaml の設定）、座標は丸める                                            |

---

## 7. 直近の Next Action

1. ~~Phase 0 を着手~~ ✅ 完了（2026-07-04）
2. ~~Phase 1（MVP: Python + ROS 2 Humble）~~ ✅ 完了（2026-07-05）
3. ~~Phase 2（多言語対応: C++ / Rust）~~ ✅ 完了（2026-07-05）
4. ~~Phase 3（ミドルウェア抽象化の本格化）~~ ✅ 完了（2026-07-05）
5. GitHub への push + CI 稼働、その後 Phase 4（配布・運用）

---

_更新履歴_

- 2026-05-20: 初版作成（要求仕様 doc/required_spec.md ベース）
- 2026-07-04: ドキュメント間整合の修正（ディレクトリ構造の実態合わせ、Phase 0 に setup.md 前提を追記、Phase 1.3 を要求仕様の出力イメージに整合、検証の Docker 前提を明記）
- 2026-07-04: Phase 0 完了を反映（雛形生成 / React Flow / lint・format / CI / dev 起動確認）
- 2026-07-04: Phase 1.1 完了を反映（モデリング基盤: ノード編集 UI・ポート・インスペクタ・型エディタ・型互換チェック）
- 2026-07-04: Phase 1.2 実装を反映（Rust モデル + save/load/new コマンド + フロント同期・メニュー。GUI 復元の手動検証待ち）
- 2026-07-04: Phase 1.2 完了（保存→終了→起動→読込の GUI 完全復元を手動確認）
- 2026-07-04: Phase 1.3 完了（コード生成エンジン + GUI 統合。Docker で colcon build・ノード起動・Pub/Sub 疎通・実装保護を検証）
- 2026-07-04: 生成レイアウトをノード完結型に変更（`<pkg>/<node>/interfaces.py` + `<node>.py`。ユーザー要望。Docker で再検証済）
- 2026-07-05: Phase 1.4 完了（examples/demo_robot.arcsyn + README クイックスタート）。**Phase 1 完了**
- 2026-07-05: Phase 2 完了（CppGenerator / RustGenerator=rclrs 採用。Rust→Python→C++ の3言語チェーンを Docker で疎通検証）
- 2026-07-05: Phase 3 完了（アダプタ generate() 主体化・レジストリ切替・GUI セレクタ・mock_pubsub 第2アダプタ・追加手順ドキュメント）
