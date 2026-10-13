<p align="center"><img src="../assets/nottio.png" alt="nottio アイコン" width="128"></p>

# nottio

[English（デフォルト）](../README.md) · 日本語 · [简体中文](readme-zh-cn.md)

付箋・図形・矢印・画像を自由に配置する、Windows / macOS向けのローカルホワイトボード。RustとGPUIで実装しています。FigJamの操作感を参考にした試作版で、共同編集や完全互換は対象外です。

## 主な操作

- **付箋**：図形ボタンの隣の付箋ボタンからドラッグして追加。クリックまたはNキーでも配置できます。初期サイズは180×140。直接入力し、Escapeで編集を終了。ドラッグで移動、ダブルクリックで再編集します。
- **図形**：四角、角丸四角、円、三角、ひし形、星、六角形。線・矢印・手描きにも対応。
- **接続**：Aキーで矢印。付箋や図形の本体から別の本体へドラッグすると接続し、移動にも追従します。直線・直角の折れ線・曲線を切り替えられます。
- **折り目と迂回**：線の途中のハンドルをドラッグして折り目を調整。始点・終点の矢印先端は個別に切り替えます。自動迂回は付箋や図形を避けますが、線同士の交差は回避しません。
- **フレーム**：付箋の隣の枠ボタン、またはShift+S。少しでも重なる内容をまとめて移動します。枠のサイズ変更では中身を拡縮しません。入れ子は未対応です。
- **画像**：キャンバスでCtrl+V（Macは⌘V）。画像本体をJSONに保存するため、再起動・コピー保存でも残ります。テキスト編集中は文章の貼り付けになるので、先にEscapeを押してください。
- 自動保存、コピー保存、ボードを開く、元に戻す・やり直す、明暗テーマ、ツールバー位置の保存。

## 起動

```sh
git clone https://github.com/eiei114/nottio.git
cd nottio
cargo run
cargo build --release
```

**Windows**：stable Rust、MSVCターゲット、Visual Studio Build ToolsのC++デスクトップ開発、Windows SDKが必要です。

**macOS**：macOS 14 Sonoma以降、Apple SiliconまたはIntel、stable Rust、Apple Command Line Tools（`xcode-select --install`）が必要です。Mac本体のCPUに合ったRustターゲットを使います。Metalシェーダーは起動時にコンパイルするため、別途Metalコンパイラーは不要です。

Finder・Dock用のアイコン付きアプリを作るにはPython 3.11以降で実行します。Pillowは不要です。

```sh
python3 scripts/build-macos-app.py
open target/macos/nottio.app
```

この`.app`はローカル用のアドホック署名です。Developer ID署名・公証済みの配布版やDMGではありません。Apple SiliconとIntelはそれぞれのMacでビルドし、Universal Binaryにはしていません。

## キー操作

WindowsはCtrl、Macは⌘でコピー・切り取り・貼り付け（C/X/V）、元に戻す（Z）、やり直す（Shift+Z）。ボードを開くはCtrl+O / ⌘O、コピー保存はCtrl+Shift+S / ⌘⇧S。Macの⌘Qは保存して終了します。接続のスナップ解除やマウスのパン操作には、MacでもControlを使います。

ツール選択：V＝選択、H＝パン、N＝付箋、R＝四角、O＝円、G＝三角、A＝矢印、X / Shift+L＝折れ線、T＝文字、Shift+S＝フレーム。

## 保存先と注意点

| OS | 作業用データ |
| --- | --- |
| Windows | `%APPDATA%\nottio\board.json` と `ui-state.json` |
| macOS | `~/Library/Application Support/nottio/board.json` と `ui-state.json` |

`NOTTIO_DATA_DIR`で保存先を指定できます。作業用ボードは最新状態に上書きするため、残したい版は「File → Save a copy…」でコピーを保存してください。画像は32 MiB、各辺8192px、1600万画素まで。SVG・アニメーション・ファイルパスの画像貼り付けは未対応です。

WindowsとMacのネイティブビルド・テストをCIで実行します。Mac実機での日本語・中国語IME、アクセシビリティ、トラックパッド、最低対応OSでの画面操作は別途確認が必要です。

## 開発とライセンス

```sh
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
python3 scripts/test-icon-assets.py   # Pillowが必要
python3 scripts/test-macos-bundle.py
```

[英語の詳細README](../README.md) / [Mac検証項目](macos-support.md) / [実装と検証の記録](verification.md)。同梱の[ホワイトボード](../vendor/gpui-whiteboard/LICENSE)、[アイコン](../vendor/gpui-whiteboard/assets/icons/LICENSE)、[フォント](../vendor/gpui-whiteboard/assets/JetBrainsMono-OFL.txt)のライセンスを保持しています。

この文書の言語切り替えは、アプリ画面の多言語化を意味しません。
