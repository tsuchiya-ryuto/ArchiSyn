# ArchiSyn

**ArchiSyn** は、ソフトウェアアーキテクチャの視覚的モデリングから、ミドルウェア接続済みのスケルトンコード生成までをシームレスにつなぐ、エンジニアのためのアーキテクチャ設計ツールです。

![ArchiSyn デモ](doc/assets/demo.gif)

_ノード追加 → プロパティ編集 → 型検索 → 接続 → 表からの型一括登録 → launch 設計_

---

## 💡 Origin of the Name

**ArchiSyn** という名前は、以下の2つの言葉を組み合わせて誕生しました。

- **Architecture**: ソフトウェアの構造、設計の根幹。
- **Synthesis (Synthesis / Synchronize)**: 抽象的な設計を具体的なコードへ「合成」し、設計と実装を「同期」させる。

「設計図を描くだけで、実装の土台が魔法のように組み上がる」という体験を象徴しています。

---

## 🚀 Key Features

- **Visual Modeling Interface**: SimulinkライクなUIで、コンポーネント、ポート、信号の流れを直感的に定義。
- **Middleware Integration**: 特定のミドルウェア（ROS 2, AUTOSAR, 各種通信スタック等）のインターフェース定義をインポートし、ノード間の接続を自動構成。
- **Skeleton Code Generation**: アーキテクチャ構成に基づき、ビルド可能なディレクトリ構造と、ボイラープレート（定型コード）を即座に出力。
- **Abstraction Layer**: ロジックの実装と、ミドルウェア固有の複雑な処理（Pub/Subの初期化など）を分離し、開発者が本来のアルゴリズムに集中できる環境を提供。

---

## 🛠 Prerequisites

ArchiSyn 本体の開発・実行に必要なもの（詳細な手順は [doc/setup.md](doc/setup.md) を参照）:

| 用途                 | 必要なもの                                                      |
| -------------------- | --------------------------------------------------------------- |
| アプリ本体           | Rust (stable) / Node.js 20+ / Tauri v2 のシステム依存パッケージ |
| 生成コードの動作検証 | Docker（`osrf/ros:humble-desktop-full` イメージ）               |

> ROS 2 Humble は Ubuntu 22.04 向けのため、Ubuntu 24.04 では Docker での検証を推奨しています。

---

## 📖 Quick Start

### 1. アプリを起動する

```bash
git clone <this-repo>
cd ArchiSyn
npm install
npm run tauri dev
```

### 2. サンプルプロジェクトを開いてコードを生成する

1. メニューの **開く** から [`examples/demo_robot.arcsyn`](examples/demo_robot.arcsyn) を読み込む
   （IMU ドライバ → センサフュージョン → コントローラの3ノード構成）
2. メニュー右の **コード生成** を押し、出力先ディレクトリ（例: `~/demo_ws`）を選ぶ
3. ROS 2 ワークスペース一式（ノードパッケージ・カスタム型 msgs・launch）が生成される

もちろん、空のキャンバスに「+ ノード追加」でゼロから設計することもできます。
ポートの型が一致しないノード同士は接続できません（型互換チェック）。

### 3. 生成されたワークスペースをビルド・実行する

```bash
docker run --rm -it -v ~/demo_ws:/ws -w /ws osrf/ros:humble-desktop-full bash -c \
  "source /opt/ros/humble/setup.bash && colcon build && \
   source install/setup.bash && ros2 launch launch/system.launch.py"
```

### 4. アルゴリズムを実装する

生成物はノードごとに完結したディレクトリになっています:

```
demo_ws/src/demo_robot_py_nodes/demo_robot_py_nodes/
└── sensor_fusion/
    ├── interfaces.py      # インターフェース部（ArchiSyn が毎回再生成。編集不可）
    └── sensor_fusion.py   # 実装部（ここにアルゴリズムを書く。再生成でも保護される）
```

`<ノード名>.py` の `on_update()`（周期処理）や `on_<入力ポート名>()`（受信フック）を
実装してください。設計を変更して再生成しても、実装部は上書きされません。

---

## 🧑‍💻 Development

```bash
npm run lint          # ESLint
npm run format:check  # Prettier
npm run build         # tsc + vite build
cd src-tauri && cargo test && cargo fmt --check  # Rust テスト・フォーマット
```

- 実装計画: [doc/plan.md](doc/plan.md)
- 要求仕様: [doc/required_spec.md](doc/required_spec.md)
- 開発環境セットアップ: [doc/setup.md](doc/setup.md)

---

## 🤝 Contribution

このプロジェクトは、設計と実装の乖離をなくし、よりクリーンなコードを素早く生み出すことを目的としています。バグ報告や機能提案、プルリクエストを歓迎します。
