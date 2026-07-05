# 新しいミドルウェアの追加手順

ArchiSyn のコード生成はミドルウェアごとの **アダプタ**（`MiddlewareAdapter` trait）に完全に委譲されています（F-7）。
新しいミドルウェア（例: ROS 2 Jazzy、Zenoh、独自プロトコル）への対応は、アダプタを1つ実装してレジストリに登録するだけです。

## 全体像

```
.arcsyn の project.middleware ─→ adapter_for(name) ─→ MiddlewareAdapter::generate()
                                     （レジストリ）        └─ ワークスペース全体を生成
```

- アダプタは「どの言語に対応するか」「パッケージ構成」「通信の配線方法」をすべて自分で決めます
- GUI のミドルウェアセレクタは `list_middlewares` コマンド経由でレジストリから自動的に選択肢を取得します

## 手順

### 1. アダプタを実装する

`src-tauri/src/codegen/middleware/<name>.rs` を作成し、trait を実装します。

```rust
use crate::codegen::GeneratedWorkspace;
use crate::model::Project;
use super::MiddlewareAdapter;

pub struct MyAdapter;

impl MiddlewareAdapter for MyAdapter {
    fn name(&self) -> &'static str {
        "my_middleware" // .arcsyn の middleware: に書く識別子
    }

    fn description(&self) -> &'static str {
        "GUI セレクタに表示する説明"
    }

    fn generate(&self, project: &Project) -> Result<GeneratedWorkspace, String> {
        // GeneratedFile の列を組み立てて返す。
        // protected: true を付けたファイル（実装部）は既存なら上書きされない（F-5）
        todo!()
    }
}
```

利用できる共通部品（`crate::codegen`）:

| 部品                                             | 役割                                                                             |
| ------------------------------------------------ | -------------------------------------------------------------------------------- |
| `build_node_names(project)`                      | ノード id → 一意な snake_case 名                                                 |
| `TopicMap::build(project, &names)`               | エッジからトピック名を解決（出力: `<node>/<port>`、入力は接続元に追従）          |
| `snake_case` / `pascal_case`                     | 命名変換                                                                         |
| `templates()`                                    | 埋め込み Tera テンプレート（`codegen/templates/` に追加し `templates()` に登録） |
| `GeneratedFile { rel_path, content, protected }` | 生成ファイル1件。`protected` は実装ファイル保護（※1）                            |

ROS 系のミドルウェア（Jazzy 追従など）であれば、既存の言語ジェネレータ
（`language::{python,cpp,rust}`）と `RosTypeResolver` trait を再利用できます。
`Ros2HumbleAdapter::generate()` がオーケストレーションの実例です。

非 ROS 系の最小実例は `MockPubSubAdapter`（`middleware/mock_pubsub.rs`）を参照してください。

### 2. レジストリに登録する

`src-tauri/src/codegen/middleware/mod.rs`:

```rust
pub mod my_middleware;

pub fn adapters() -> Vec<Box<dyn MiddlewareAdapter>> {
    vec![
        Box::new(ros2_humble::Ros2HumbleAdapter),
        Box::new(mock_pubsub::MockPubSubAdapter),
        Box::new(my_middleware::MyAdapter), // ← 追加
    ]
}
```

これだけで GUI のセレクタに現れ、`middleware: my_middleware` の .arcsyn から生成できるようになります。

### 3. テストを書く

`src-tauri/tests/codegen_integration.rs` に倣い、最低限:

- 期待するファイルレイアウトが生成されること
- エッジ由来のトピック配線が正しいこと
- 実装部が `protected` であること（再生成でユーザーコードが守られること）

### 4. 検証する

生成物が実際にビルド・実行できることを確認します（ROS 系なら Docker で colcon build、
`doc/setup.md` B.1 / `docker/humble-rust.Dockerfile` 参照）。

## 設計上の約束事

- **実装ファイル保護（※1 / F-5）**: ユーザーが書く実装部は必ず `protected: true` にする。
  インターフェース部（配線）は毎回再生成し、実装部と分離する
- **ノード完結型レイアウト**: ノードごとのディレクトリに interfaces + 実装部を同居させる
  （要求仕様「出力イメージ」参照）
- **トピック規約**: `TopicMap` を使い、出力 `<node>/<port>`・入力は接続元追従に揃える
  （ミドルウェアを切り替えても配線の意味が変わらないようにする）

---

_更新履歴_

- 2026-07-05: 初版作成（Phase 3）
