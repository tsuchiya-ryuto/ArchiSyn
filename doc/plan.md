# ArchiSyn 実装計画 (plan.md)

本ドキュメントは `doc/required_spec.md` を入力として、ArchiSyn の実装計画をまとめたものです。
ユーザーとの合意済み技術選定に基づき、MVP からのフェーズ分けで進めます。

---

## 1. 技術選定（合意済み）

| 項目 | 採用技術 | 理由 |
|------|---------|------|
| デスクトップアプリ | **Tauri (Rust + Web)** | 軽量・高速。Rustでコード生成ロジックを書けばサポート言語の1つ（Rust）と相性◎ |
| ノードグラフUI | **React Flow (xyflow)** | ノードベースエディタのデファクト。カスタマイズ性が高い |
| フロントエンド | **React + TypeScript + Vite** | React Flow との親和性 |
| ファイル形式 | **YAML** (`.arcsyn`) | テキスト・Git親和・コメント可。GUI完全復元に必要な全状態を含める |
| コード生成 | **Rust (Tauriバックエンド側)** + **テンプレートエンジン (Tera)** | Tauriの`#[tauri::command]`から呼び出し可能 |
| ミドルウェア | **ROS 2 Humble** | 抽象化レイヤ経由（Phase 3 で本格対応） |
| 対象OS | Linux / macOS / Windows | Tauri が標準対応 |

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
  middleware: ros2_humble       # 将来切替可

custom_types:                    # F-3
  - name: FusedPose
    fields:
      - { name: position, type: geometry_msgs/Vector3 }
      - { name: confidence, type: float64 }

nodes:                           # F-1, F-2, F-4, F-6
  - id: n_sensor_fusion
    label: SensorFusion
    language: cpp                # python | cpp | rust
    period_ms: 50                # 実行周期
    position: { x: 120, y: 200 } # GUI 復元用
    size:     { w: 180, h: 80 }
    inputs:
      - { name: imu,   type: sensor_msgs/Imu }
      - { name: lidar, type: sensor_msgs/LaserScan }
    outputs:
      - { name: fused, type: FusedPose }
    params:
      - { name: alpha, type: float64, default: 0.7 }

edges:                           # F-2
  - id: e1
    source: { node: n_sensor_fusion, port: fused }
    target: { node: n_controller,    port: pose }

viewport:                        # GUI 完全復元
  zoom: 1.0
  pan:  { x: 0, y: 0 }
```

---

## 5. フェーズ別実装計画

### Phase 0: プロジェクト初期化（1〜2日）

> **前提**: `doc/setup.md` のチェックリスト（E）を完了していること。特に A.6 の hello-tauri 起動確認まで済ませてから着手する。

- [ ] `cargo create-tauri-app` で Tauri + React + TypeScript 雛形生成
- [ ] React Flow 導入・最小ノード表示確認
- [ ] CI（GitHub Actions: build & test）
- [ ] `.gitignore`, lint/format (rustfmt, eslint, prettier)
- [ ] **マイルストーン**: `npm run tauri dev` で空キャンバスが起動

### Phase 1: MVP — Python + ROS 2 Humble（2〜3週）

最小の「設計 → ビルド可能なROS 2パッケージ生成」フローを通すフェーズ。

#### 1.1 モデリング基盤
- [ ] React Flow にノード追加・削除・接続・移動
- [ ] ノードのポート（入出力）UI
- [ ] サイドパネル: ノードのプロパティ編集（label, period_ms, params）
- [ ] カスタム型エディタ（最小：フィールド名・型のみ）
- [ ] 型互換チェック（エッジ接続時のバリデーション）

#### 1.2 .arcsyn ファイル I/O
- [ ] Rust 側に `Project` 構造体 + `serde_yaml`
- [ ] Tauri command: `save_project`, `load_project`, `new_project`
- [ ] フロントとの状態同期（Zustand）
- [ ] **検証**: 保存→終了→起動→読込で完全復元できる

#### 1.3 Python + ROS 2 コード生成

要求仕様の「出力イメージ」に従い、言語別パッケージ構成（`<project>_py_nodes` / `<project>_msgs`）で生成する。
F-5（実行処理部とインターフェース部の分離）は「インターフェース部＝毎回再生成、実装部スケルトン＝既存時は保護（※1）」で実現する。

- [ ] `LanguageGenerator` / `MiddlewareAdapter` trait 定義
- [ ] `PythonGenerator` + `Ros2HumbleAdapter` 実装
- [ ] Tera テンプレート（`<project>_py_nodes` パッケージ）:
  - `package.xml`, `setup.py`
  - `interfaces/<project>_py_nodes_interfaces.py`（インターフェース部。毎回再生成）
  - `<project>_py_nodes/<node>.py`（実装部スケルトン。rclpy Node, subscribe/publish, timer）
- [ ] カスタム型 → `<project>_msgs/msg/*.msg` 生成（共通パッケージ）
- [ ] `launch/system.launch.py` 生成
- [ ] **実装ファイル保護（※1）**: 実装部の既存ファイルは上書きしない（F-5）
- [ ] **検証**: 生成 → `colcon build` 成功 → ノード起動確認
  - 開発機は Ubuntu 24.04 のため ROS 2 Humble はネイティブ非対応。検証は `osrf/ros:humble-desktop-full` コンテナ内で実施（`doc/setup.md` B.1 参照）

#### 1.4 ドキュメント・サンプル
- [ ] サンプル `.arcsyn`（2〜3ノード）
- [ ] README にクイックスタート追記

**Phase 1 完了条件**: GUI で 2 ノードを Pub/Sub 接続し、生成された ROS 2 ワークスペースがビルド・実行できる。

---

### Phase 2: 多言語対応（2週）

- [ ] `CppGenerator` + テンプレート（`rclcpp`, CMakeLists.txt）
- [ ] `RustGenerator` + テンプレート（`r2r` または `ros2_rust`）
- [ ] ノード単位の言語切替 UI（F-6）
- [ ] 異言語ノード間の型整合（`my_robot_msgs` 共通パッケージで担保）
- [ ] **検証**: C++/Python 混在ワークスペースで Pub/Sub 動作（Phase 1 と同様に Docker コンテナ内で実施。`doc/setup.md` B.1 参照）

---

### Phase 3: ミドルウェア抽象化の本格化（1〜2週）

- [ ] `MiddlewareAdapter` インタフェースの再点検
  - サブスクライバ生成、パブリッシャ生成、周期タイマ、型マッピング
- [ ] プロジェクトレベルの `middleware:` フィールドで切替
- [ ] ドキュメント: 新しいミドルウェアの追加手順
- [ ] （任意）スタブの第2アダプタ（例：純pub/sub mock）

---

### Phase 4: 配布・運用（1週）

- [ ] 各 OS 向けインストーラ（Tauri bundle: `.deb` / `.dmg` / `.msi`）
- [ ] アンインストーラ動作確認
- [ ] バージョニング・リリースワークフロー（GitHub Releases）
- [ ] ユーザーマニュアル

---

## 6. リスクと対応

| リスク | 対応 |
|--------|------|
| React Flow のカスタムノードで Simulink 風 UX を出し切れない | Phase 1 で許容範囲を確認、不足ならカスタムレンダラで補強 |
| ros2_rust の安定性 | Phase 2 で評価。難があれば Rust 対応は Phase 2 後半に後ろ倒し |
| 既存実装ファイルの保護で edge case（リネーム検知等） | Phase 1 では「ファイル名一致なら保護」のシンプル方針。Phase 3 でメタデータ管理を検討 |
| YAML の差分が大きくなる | キー順序を安定化（serde_yaml の設定）、座標は丸める |

---

## 7. 直近の Next Action

1. Phase 0 を着手：`cargo create-tauri-app` で雛形作成 → 最小 React Flow 表示 → 初回コミット
2. Phase 1.1 〜 1.3 を順次実装、こまめにコミット

---

*更新履歴*
- 2026-05-20: 初版作成（要求仕様 doc/required_spec.md ベース）
- 2026-07-04: ドキュメント間整合の修正（ディレクトリ構造の実態合わせ、Phase 0 に setup.md 前提を追記、Phase 1.3 を要求仕様の出力イメージに整合、検証の Docker 前提を明記）
