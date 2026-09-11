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
