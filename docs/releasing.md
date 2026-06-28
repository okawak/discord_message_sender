# リリース手順

## リリースフロー

v0.3以降は、次の順序でリリースします。

1. 機能ブランチを`develop`へPRでマージする
2. `develop`から`main`へのリリースPRをレビューしてマージする
3. GitHub Actionsの`Release Plugin`を`main`から手動実行する
4. workflowがversion更新コミット、tag、GitHub Releaseを順番に作成する

versionファイルは開発中に次期versionへ変更しません。リリースworkflowへ入力したversionを、CIが一括設定します。

## 実行前の確認

- `develop`から`main`へのPRがレビュー済みである
- `main`のCIが成功している
- リリースversionが`0.3.0`のような`x.y.z`形式である
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

## version更新対象

version更新処理は`scripts/prepare-release.ts`へ集約されています。

- `package.json`
- `manifest.json`
- `Cargo.toml`
- `Cargo.lock`
- `versions.json`

`manifest.json`の`minAppVersion`が、`versions.json`の新しいversionへ設定されます。

## 完了確認

- workflowの全stepが成功している
- `main`に`chore: release <version>`コミットが追加されている
- tagとversion更新コミットのSHAが一致している
- GitHub Releaseに`manifest.json`、`main.js`、ZIPがある
- `main`の`manifest.json`がリリースversionになっている

CLIでは次のように確認できます。

```bash
gh release view 0.3.0
git fetch --tags
git rev-parse 0.3.0^{}
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
