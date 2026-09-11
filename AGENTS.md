# Repository Instructions

## General policy

- 日本語で簡潔かつ丁寧に回答してください。

## Pull request policy

- PRタイトルに`[codex]`プレフィックスを付けないでください。
- PRタイトルは英語で記述してください。

## Pull request review cycle

1. PRを作成したら、PRへ`@codex review`とコメントしてCodexへレビューを依頼してください。
2. CIとCodexレビューが完了するまで待機してください。
3. CIの失敗またはレビューコメントがあれば、内容を確認してください。
4. 要対応と判断した指摘へ対応してください。
5. 対応済みのレビューコメントをresolveしてください。
6. 修正をpushし、再度`@codex review`とコメントして手順2へ戻ってください。
7. CIが成功し、対応すべきレビューコメントがなくなった場合にのみPRをマージしてください。
8. マージ後にマージ元ブランチを削除してください。

## Commit signing policy

- ローカルまたは人が作成するコミットは署名付きにしてください。
- 署名されていないコミットを作る可能性がある操作の前に、署名設定を確認してください。
- `.github/workflows/release.yml`が`github-actions[bot]`として作成するrelease versionコミットは、自動化用の例外として署名なしを許可します。
- release workflow以外のCIコミットには、この例外を適用しません。
