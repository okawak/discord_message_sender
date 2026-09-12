# RustとTypeScriptの役割

Rustで日常的にロジックを読んで変更できるよう、ObsidianのI/Oから独立した処理を`crates/parse_message/src/core/`へまとめています。目的は実装言語の統一と保守のしやすさです。

## コードの入口

| ファイル | 担当 |
| --- | --- |
| `core/models.rs` | 設定・Discordメッセージ・処理結果のデータ型 |
| `core/settings.rs` | 初期値、旧設定の移行、入力値の正規化、同期用スナップショット |
| `core/channels.rs` | チャンネル名の検証、保存先の生成、Unicode正規化を含む重複判定 |
| `core/dates.rs` | RFC 3339日時の解析、タイムゾーン変換、ISO週番号、既存ログの探索対象日 |
| `core/messages.rs`、`command.rs` | メッセージとURLコマンドの解析、投稿者・ファイル名の生成 |
| `core/logs.rs` | ログの識別、日次・週次・月次ログの結合、管理マーカー、重複排除 |
| `core/storage.rs` | 既存ログの探索パス、個別ファイルと集約ログへの保存計画 |
| `core/sync.rs` | 同期前の検証、新着ページの選別、古い順への並べ替え、通知文の選択 |
| `core/discord.rs` | APIパス、レート制限、再試行判断、エラー文・通知文の生成 |
| `bindings.rs` | 型付きWASM関数の公開、JS値との変換と例外への変換 |

HTMLからMarkdownへの変換は引き続き`crates/html_to_markdown`が担当します。

## TypeScriptに残す処理

実行時のTSは次の8ファイルです。単にRustの関数や型を再公開するファイルは置かず、呼び出し元から生成済みの`pkg/parse_message.js`を直接インポートします。

- `main.ts`: Obsidianプラグインの起動、コマンド登録、同期の排他、設定の保存、`Intl`からOSのタイムゾーン名を取得
- `settingTab.ts`: Obsidian 1.13の設定画面、入力イベントと通知
- `settings.ts`: 保存データのJS値への対応、設定変更時のJSオブジェクト参照の維持
- `discordApi.ts`: Obsidian `requestUrl`によるDiscord通信、待機、通信例外の捕捉
- `vault.ts`: Vaultのファイル探索・読み込み・作成・`Vault.process`による更新
- `channelSync.ts`: ページ取得・メッセージ変換・保存・通知の非同期実行順序と失敗時の処理
- `wasmCore.ts`: 非同期WASM初期化と再試行、Rustが生成する通信エラーのJS例外への対応
- `wasmBridge.ts`: 初期化失敗のObsidian通知、クリッピング用のURL取得とRust変換処理の接続

ビルド・リリース用スクリプトもBun/Nodeのホスト処理としてTypeScriptに残します。

## 境界のルール

1. Rustの型を`serde`と`tsify`で定義し、`wasm-bindgen`で`pkg/parse_message.d.ts`を生成します。TS側で同じデータ型を手書きしません。
2. `bindings.rs`は`Ts<T>`を使い、`to_rust()`と`into_ts()`の失敗を通常の`Result`として処理します。ロジックは`core/`へ置きます。
3. WASM関数をモジュールの読み込み時やプラグインのコンストラクターから呼びません。`onload()`で初期化を待ってから設定を読み込みます。
4. Rustから戻る値はコピーです。同期中のカーソル更新が既存設定へ届くよう、チャンネルはRustが返すインデックスで元のJSオブジェクトを参照します。設定画面で別項目を編集してもチャンネル配列を置き換えません。
5. 保存計画はRustで作りますが、既存ログの結合は`Vault.process`のコールバック内で最新の本文に対して実行します。保存の完了後に同期カーソルを進めます。
6. WASM版のタイムゾーン変換はホストの`Intl.DateTimeFormat`、Unicode正規化は`String.normalize("NFC")`と`toLowerCase()`をRustから呼び出します。日付の解析・整形・ISO週の計算や保存先の判定はRustに残します。規則は実行環境のIANA/Unicodeデータに従い、プラグインのlockfileでは固定しません。ネイティブテストには`chrono-tz`と`unicode-normalization`を使います。
7. Discord IDは文字列で保持し、新旧比較をRustで行います。JSの浮動小数点数には変換しません。

## 依存の管理と選定

外部ライブラリのバージョンと基本featuresはルートの`Cargo.toml`の`[workspace.dependencies]`で管理します。各crateは必要なものだけを`dependency-name.workspace = true`で参照します。ルートに定義しただけでは各crateへの依存にはなりません。テスト用の外部依存もルートで定義し、利用側の`[dev-dependencies]`で参照します。方式は[Cargo公式のworkspace dependencies](https://doc.rust-lang.org/cargo/reference/workspaces.html#the-dependencies-table)に従います。workspace内のcrateへの依存は利用側に相対パスで記述し、`parse_message`では`html_to_markdown = { path = "../html_to_markdown" }`とします。

ログマーカー・ファイル名・通知変数の照合には`regex-lite`を使います。エラー型には引き続き`thiserror`を使います。エラー文、CRLF/BOM、ASCII数字の判定、通知の置換値を再展開しない動作を維持します。

| 維持する依存 | 用途・理由 |
| --- | --- |
| `thiserror` | `Display`／`Error`の実装をコンパイル時に生成し、エラー型の見通しを保つ |
| `html5ever` | HTMLの構文解析、壊れたHTMLの回復、文字参照など。HTML仕様全体の独自実装を避ける |
| `regex-lite` | 固定形式のログ・ID・テンプレートの照合。専用パーサーの保守を避ける |
| `chrono` | RFC 3339、暦・ISO週、日付の整形。default featuresを無効化して`std`だけを指定 |
| `chrono-tz` | ネイティブビルド専用のタイムゾーン変換。WASM版はホストの`Intl`を使い、データを同梱しない |
| `unicode-normalization` | ネイティブビルド専用のNFC正規化。WASM版ではホストのUnicode実装を使う |
| `serde`、`serde_json` | 設定の移行・検証、DiscordレスポンスのJSON処理 |
| `wasm-bindgen`、`js-sys`、`serde-wasm-bindgen`、`tsify` | JS/WASM間の呼び出し・型変換・TypeScript型の生成。`tsify`は`js` featureだけを有効化 |
| `html_to_markdown` | workspace内のHTML変換crate |
| `pretty_assertions`、`rstest`、`indoc` | HTML変換のテスト専用。releaseのWASMには組み込まれない |

依存の宣言数と配布サイズは同じではありません。機能・テーブルを含むライブラリと、コンパイル時にコードを生成するマクロ、テスト専用の依存を分けて判断します。`thiserror`は[公式ドキュメント](https://docs.rs/thiserror/latest/thiserror/)でも手書きの`Error`実装に相当すると説明されており、配布サイズ削減のために外す対象とはしません。

## 配布サイズ

非圧縮サイズの比較ビルド、採用した改善、HTMLパーサーなどの候補は[サイズ調査](wasm-size-investigation.md)に記録しています。

配布サイズは[サイズ調査](wasm-size-investigation.md)を参考に継続して確認します。Viteが表示する転送時gzipサイズと、実際に配布する非圧縮`main.js`のサイズは別です。

- `vite.config.ts`は生成済みWASMを**圧縮せず**base64として`main.js`に埋め込みます。base64は文字列への符号化であり、gzip等の圧縮ではありません。
- 初期化時に標準APIの`atob`でWASMのバイト列へ戻します。追加の展開ライブラリ、外部アセット取得、Node固有APIは不要です。
- 大きなIANA/Unicodeテーブルはホストの標準APIを利用して同梱を省きます。Rustの処理を独自の簡易パーサーへ置き換えてサイズを削る方針は採りません。
- ホスト側のIANA/Unicode更新とネイティブテスト用ライブラリの更新時期は異なる場合があります。実WASMでも互換性fixture、夏時間・分単位の時差・Unicodeの重複判定を検証します。

利用する標準API: [Intl.DateTimeFormat.formatToParts](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/DateTimeFormat/formatToParts)、[String.normalize](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/String/normalize)。

## 検証

```bash
# Rustの変更後はWASMとTS型を再生成
bun run build
bun run type-check
bun run check
bun run test
bun run test:wasm
bun run verify:build
```

Rustのロジックだけを確認する場合は`cargo test -p parse_message --locked`を実行します。Bunテストは`bunfig.toml`のpreloadで実際のWASMを初期化し、TSアダプターとRustの結合を検証します。古い`pkg`でテストしないよう、Rust変更後は先にビルドしてください。

モジュールの入口は`src/core.rs`、子モジュールは`src/core/`に置きます。`mod.rs`も有効な形式ですが、[Rust公式が推奨する命名](https://doc.rust-lang.org/reference/items/modules.html#module-source-filenames)に合わせ、ファイル名から担当モジュールが分かる形に統一します。

通常の単体テストは対象の実装ファイル内の`#[cfg(test)] mod tests`に置きます。実装とテストを一緒に読めるようにするためで、配布サイズを削るための分離ではありません。別ファイルに置いた場合も、同じ`cfg(test)`でテストコードを本番ビルドから除外できます。[Rust公式のテスト配置](https://doc.rust-lang.org/book/ch11-03-test-organization.html)

crate直下の`tests/compatibility.rs`には、移行前のTypeScript出力を記録した`tests/fixtures/compatibility.json`と比較する4件の回帰テストをまとめます。Cargoの統合テストとして独立したcrateから`parse_message::core`の公開APIを呼び出します。このためライブラリの`crate-type`には、WASM配布用の`cdylib`に加えてRustからリンクするための`rlib`を指定します。`rlib`は配布物へ同梱しません。比較対象は設定、Unicodeパス、ログのバイト列、夏時間・うるう日・ISO週番号です。期待値は新しいRust実装から再生成せず、既存の保存形式を保護するデータとして扱います。

WASMの生成には`wasm-pack`を使います。Cargo、wasm-bindgen、wasm-optを個別に呼び分けず、`package.json`の`wasm:build`を単一の入口にします。`Cargo.toml`ではwasm-packの標準設定で`wasm-opt -Oz`を指定します。`verify:build`は別ディレクトリで再ビルドして成果物を比較します。

開発環境では、Rust toolchainに加えて`wasm-pack`を準備します。これらのビルドツールはプラグインに同梱しません。

```bash
cargo install wasm-pack --version 0.15.0 --locked
bun install --frozen-lockfile
bun run build
```

Cargoのバイナリディレクトリ（通常は`$HOME/.cargo/bin`、`CARGO_HOME`指定時はその`bin`）をPATHへ追加してください。CIでは固定versionの`wasm-pack`をインストールします。

`tests/wasmBoundary.test.ts`では型変換失敗後の継続動作・メモリ・オブジェクト参照・ホストの日時変換と正規化を検証します。`tests/bundle.smoke.ts`では生成済みCommonJSの`dist/main.js`を独立したJSコンテキストで読み込み、プラグイン生成、WASM初期化、旧設定の移行、設定保存まで検証します。Obsidian APIはモックです。全fetchを禁止し、`DecompressionStream`とNode固有APIのない環境で起動できること、埋め込んだbase64の復号結果と初期化に渡すWASMがそれぞれ生成元とバイト単位で一致すること、初期化が1回だけであることも確認します。

参考: [tsifyの型生成とTs<T>](https://docs.rs/tsify/latest/tsify/)、[wasm-bindgenのSerde連携](https://rustwasm.github.io/docs/wasm-bindgen/reference/arbitrary-data-with-serde.html)
