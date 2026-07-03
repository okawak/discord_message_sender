# リリース手順

## リリースフロー

次の順序でリリースします。

1. リリースサイクルの開始時に最新の`main`から`develop`を作成・更新する
2. `feat/*`、`refactor/*`、`docs/*`ブランチから`develop`へPRを作成する
3. 各PRをレビューし、確認できたものだけを`develop`へマージする
4. `develop`から`main`へのリリースPRをレビューしてマージする
5. `main`のCI成功後、GitHub Actionsの`Release Plugin`を`main`から手動実行する
6. workflowがversion更新コミット、tag、GitHub Releaseを順番に作成する

versionファイルは開発中に次期versionへ変更しません。リリースworkflowへ入力したversionを、CIが一括設定します。

## 実行前の確認

- `develop`から`main`へのPRがレビュー済みである
- `main`のCIが成功している
- リリースversionが`0.4.0`のような`x.y.z`形式である
- versionに`v`接頭辞やprerelease文字列を付けていない
- 同じversionのtagが別のコミットを指していない
- GitHub Actionsに`contents: write`権限と`main`へのpush権限がある

`main`へbranch protectionを追加する場合は、release workflowのbot pushを許可する必要があります。

## GitHub Actionsからの実行

1. GitHubの**Actions**を開く
2. **Release Plugin**を選択する
3. **Run workflow**を選択する
4. Branchに`main`を指定する
5. `version`へリリースversionを入力する
6. **Run workflow**を実行する

workflowは次の処理を行います。

1. version形式と実行ブランチを検証する
2. `package.json`、`manifest.json`、`Cargo.toml`、`Cargo.lock`を更新する
3. `versions.json`へObsidianの最低対応versionを追加する
4. TypeScript、Biome、Clippy、Rust/TypeScriptテストを実行する
5. production buildとrelease archiveを作成する
6. `github-actions[bot]`でversion更新コミットを`main`へpushする
7. 同じコミットへversion名のannotated tagを作成する
8. `manifest.json`、`main.js`、ZIPをGitHub Releaseへ添付する

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

## v0.4.0の受入確認

自動テストに加えて、リリースPRをマージする前にデスクトップ版Obsidianで次の項目を確認します。確認結果はリリースPRへ記録します。

- [ ] v0.3系の`data.json`が、チャンネルと`lastProcessedMessageId`を維持して移行される
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
- `main`に`chore: release <version>`コミットが追加されている
- tagとversion更新コミットのSHAが一致している
- GitHub Releaseに`manifest.json`、`main.js`、ZIPがある
- `main`の`manifest.json`がリリースversionになっている

CLIでは次のように確認できます。

```bash
gh release view 0.4.0
git fetch --tags
git rev-parse 0.4.0^{}
git rev-parse origin/main
```

## 失敗時の対応

同じversionでworkflowを再実行できます。

- versionコミット前の失敗: 修正後に同じversionで再実行する
- versionコミット後、tag作成前の失敗: `main`が進んでいなければ、同じversionで再実行してreleaseコミットを再利用する
- tag作成後、Release作成前の失敗: tagが同じコミットなら再利用してReleaseを作成する
- tagが別のコミットを指す場合: workflowは停止する。tagを自動更新せず、履歴を確認してから手動対応する
- versionコミット後に`main`が更新された場合: workflowは停止する。releaseコミットと追加変更を確認してから手動対応する
- 実行中に別の変更が`main`へpushされ、versionコミットのpushに失敗した場合: 最新の`main`から再実行する

versionの巻き戻しはworkflowで禁止されています。
