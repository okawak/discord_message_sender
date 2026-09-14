# リリース手順

## リリースフロー

次の順序でリリースします。

1. リリースサイクルの開始時に最新の`main`から`develop`を作成・更新する
2. `feat/*`、`refactor/*`、`docs/*`ブランチから`develop`へPRを作成する
3. 各PRをレビューし、確認できたものだけを`develop`へマージする
4. `develop`から`main`へのリリースPRをレビューしてマージする
5. `main`のCI成功後、GitHub Actionsの`Release Plugin`を`main`から手動実行する
6. workflowがversion更新コミットとtagを作成し、tagからGitHub Releaseを作成する

versionファイルは開発中に次期versionへ変更しません。リリースworkflowへ入力したversionを、CIが一括設定します。

## 実行前の確認

- `develop`から`main`へのPRがレビュー済みである
- `main`のCIが成功している
- リリースversionが`0.4.0`のような`x.y.z`形式である
- versionに`v`接頭辞やprerelease文字列を付けていない
- 同じversionのtagが別のコミットを指していない
- version更新用jobに`actions: write`と`contents: write`、公開用jobに`attestations: write`、`contents: write`、`id-token: write`が設定されている
- version更新用jobが`main`へpushできる

`main`へbranch protectionを追加する場合は、release workflowのbot pushを許可する必要があります。
手動workflowの実行権限はGitHubのrepository write権限で管理します。リリース担当者以外へwrite権限を付与する場合は、Actionsの実行権限も併せて見直してください。

## GitHub Actionsからの実行

1. GitHubの**Actions**を開く
2. **Release Plugin**を選択する
3. **Run workflow**を選択する
4. Branchに`main`を指定する
5. `version`へリリースversionを入力する
6. **Run workflow**を実行する

`Release Plugin`は次の処理を行います。

1. version形式と実行ブランチを検証する
2. `package.json`、`manifest.json`、`Cargo.toml`、`Cargo.lock`を更新する
3. `versions.json`へObsidianの最低対応versionを追加する
4. Bun依存関係の監査、TypeScript、Biome、Clippy、Rust/TypeScriptテストを実行する
5. production buildとWASM smoke testを実行する
6. `github-actions[bot]`でversion更新コミットを`main`へpushする
7. 同じコミットへversion名のannotated tagを作成する
8. tagを指定して同じ`Release Plugin` workflowを自動実行する

tagから実行された`Release Plugin`は、tagのソースを改めてcheckoutし、固定したツールチェーンでテストとbuildを再実行します。生成した`main.js`と`manifest.json`へGitHub artifact attestationを付与し、この2ファイルだけをGitHub Releaseへ添付します。

Rustのversionは`rust-toolchain.toml`、Bunのversionは`.bun-version`、Nodeのversionは`.node-version`を参照します。`wasm-pack`はworkflow内で固定します。これらのversionと依存関係はRenovateが週次で更新PRを作成するため、CIを確認してからマージしてください。GitHub Actionsは`@v7`のようなmajor tagを維持します。

Renovateを動作させるには、repositoryへRenovate GitHub Appをインストールする必要があります。設定は`renovate.json`へ集約し、次を更新対象とします。

- Bun packageと`bun.lock`
- Cargo crateと`Cargo.lock`
- Rust toolchain
- Bun runtime
- Node runtime
- wasm-pack
- GitHub Actions

固定versionは、tagから同じ環境で再ビルドできるようにするために維持します。更新作業は手動編集せず、RenovateのPRとしてレビューします。

CIが作成するversion更新コミットとtagはGPG署名されません。人が作成する通常のコミットは、引き続き署名必須です。

`Cargo.lock`は手動でversionを書き換えません。`scripts/prepare-release.ts`が`Cargo.toml`を更新した後に`cargo update`を実行し、その結果をworkflowがversion更新コミットへ含めます。

## version更新対象

version更新処理は`scripts/prepare-release.ts`へ集約されています。

- `package.json`
- `manifest.json`
- `Cargo.toml`
- `Cargo.lock`
- `versions.json`

`manifest.json`の`minAppVersion`が、`versions.json`の新しいversionへ設定されます。

`0.5.0`は採番済みで、最低対応版は次のように記録されています。

```json
"0.5.0": "1.13.0"
```

今後の採番では、[公式サンプルのversion更新処理](https://github.com/obsidianmd/obsidian-sample-plugin/blob/master/version-bump.mjs)と同じく、`manifest.json`の現在の`minAppVersion`を新しいversionの値として`versions.json`へ追加します。`0.4.0`以前のエントリーは`1.8.10`、`0.5.0`は`1.13.0`のまま維持します。既存versionの最低対応版が異なる場合、採番処理は上書きせず失敗します。同じversionと最低対応版での再実行は可能です。

`Release Plugin`へ新しいversionを指定すると、package・manifest・Cargoのversion更新と`versions.json`へのエントリー追加をまとめて実行します。開発中のversionファイルは採番前の値を維持します。

Rust/TypeScriptの役割と開発時の検証方法は[実装構成](rust-core.md)を参照してください。

## Obsidian 1.13以降への対応とビルド再現性

`0.5.0`以降の最低対応版はObsidian 1.13.0です。設定画面は[Obsidian公式の移行ガイド](https://docs.obsidian.md/plugins/guides/migrate-declarative-settings)に従い、`getSettingDefinitions()`のみで描画します。公開済みリリースの`versions.json`エントリーは変更せず、最低対応版を変更する場合は新しいプラグインversionとして追加します。

CIとリリースはUbuntu 24.04と固定したNode/Bun/Rust/wasm-packを使います。`bun install --frozen-lockfile`とCargoの`--locked`で依存関係の意図しない再解決を防ぎます。Viteの出力ターゲットはES2022です。WASMは圧縮せずbase64として`main.js`に埋め込みます。

```bash
bun install --frozen-lockfile
bun run build
bun run verify:build
```

`scripts/build-wasm.ts`はcheckout、Cargo home、Rust標準ライブラリのソースパスを`--remap-path-prefix`で共通表記へ変換します。これらのパスはpanic位置などとしてWASMに残るため、ツールのversion固定だけでは異なる環境で同じ成果物になりません。スクリプトはパスの正規化とwasm-packの呼び出しだけを担当します。コマンド実行・出力の取得・失敗時の停止は既存のBunに含まれる[Bun Shell](https://bun.sh/docs/runtime/shell)へ任せています。wasm-packが実行するBinaryenは`bun.lock`に固定し、`bun run wasm:build`が設定するPATHから選択します。

`verify:build`はcheckoutとCargo homeを別の一時パスへ移し、Rust/WASMをコンパイルキャッシュなしでビルドして、`dist/main.js`と`dist/manifest.json`をバイト単位で比較します。ダウンロード済みCargo依存と固定済みJavaScript依存は再利用します。不一致がある場合はCIとリリースを停止します。この検証は同じOS・ツールチェーン内の比較であり、任意の別環境や過去のリリースとの一致を保証するものではありません。公開済みの添付ファイルは上書きせず、修正を含む新しいversionをリリースしてください。

初回の`rustc`呼び出しでは、rustupが`rust-toolchain.toml`に指定されたツールチェーンを自動インストールする場合があります。`build-wasm.ts`はRust環境の問い合わせを直列に実行し、複数のrustupプロセスによるダウンロードの競合を避けます。0.5.3の外部ビルド検証で発生した`clippy`の`.partial`ファイル消失は、この競合によるものです（[rustupの既知の問題](https://github.com/rust-lang/rustup/issues/988)）。CIでは通常の事前インストール済み環境に加え、空の`RUSTUP_HOME`と別のコンパイル先で`bun run build`を実行し、両方の成果物が一致することも検証します。既存のCargo依存とJavaScript依存は再利用します。

0.5.2のレビューで示されたネットワーク呼び出し、base64復号、WASMメモリexportは、同期やWASMの実行に必要な機能の開示です。スキャンやbuild verificationの「not available」は審査側の実行状況も関係し、コード変更だけで解消するとは限りません。新しいリリースで再審査した結果を確認してください。

リリース前にObsidian 1.13以降で設定検索、チャンネルの追加・削除、設定の再読み込み、Bot tokenの表示切替を確認してください。

## リリース前の受入確認

自動テストに加えて、リリースPRをマージする前にデスクトップ版Obsidianで次の項目を確認します。確認結果はリリースPRへ記録します。

- [ ] 旧形式の`data.json`が、チャンネルと`lastProcessedMessageId`を維持して移行される
- [ ] 1メッセージ1ファイル、日次、週次、月次の各形式で保存できる
- [ ] 日次ログは日付が見出し1になり、週次・月次ログは日付が見出し2になる
- [ ] 投稿者名・投稿時刻の各トグルがまとめたログへ反映される
- [ ] 端末のローカルタイムゾーンでファイル名、日付、時刻が決まる
- [ ] 保存形式を途中で変更しても既存ファイルは維持され、新着だけが新形式へ保存される
- [ ] 同期を再試行しても同じメッセージが重複しない
- [ ] 複数チャンネルのログが別フォルダへ保存される
- [ ] 100件を超える新着がページ分割され、古い順に保存される
- [ ] `!url`のクリッピングは個別ファイルとして保存される
- [ ] 同期通知を無効にするとDiscordへ通知を送信しない

自動確認は次のコマンドを実行します。

```bash
bun run type-check
bun run check
bun run test
bun run test:wasm
bun run build
```

## 完了確認

- workflowの全stepが成功している
- tagから自動実行された`Release Plugin`のpublish jobが成功している
- `main`に`chore: release <version>`コミットが追加されている
- tagとversion更新コミットのSHAが一致している
- GitHub Releaseに`manifest.json`と`main.js`だけがある
- `main.js`と`manifest.json`のartifact attestationを検証できる
- `main`の`manifest.json`がリリースversionになっている

CLIでは次のように確認できます。

```bash
release_version=0.5.0 # 確認するversionへ置き換える
gh release view "$release_version"
gh release download "$release_version" --pattern main.js
gh attestation verify main.js --repo okawak/discord_message_sender
git fetch --tags
git rev-parse "${release_version}^{}"
git rev-parse origin/main
```

## 失敗時の対応

同じversionでworkflowを再実行できます。

- versionコミット前の失敗: 修正後に同じversionで再実行する
- versionコミット後、tag作成前の失敗: `main`が進んでいなければ、同じversionで再実行してreleaseコミットを再利用する
- tag作成後、publication実行前の失敗: `Release Plugin`を同じversionで再実行し、tagを再利用してpublicationを再依頼する
- publish jobの失敗: tagから`Release Plugin`を再実行するか、`main`から同じversionで再実行する
- tagが別のコミットを指す場合: workflowは停止する。tagを自動更新せず、履歴を確認してから手動対応する
- versionコミット後に`main`が更新された場合: workflowは停止する。releaseコミットと追加変更を確認してから手動対応する
- 実行中に別の変更が`main`へpushされ、versionコミットのpushに失敗した場合: 最新の`main`から再実行する

versionの巻き戻しはworkflowで禁止されています。
