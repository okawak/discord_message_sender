[English](../README.md) | 日本語 | [简体中文](./README.zh-CN.md)

# discord_message_sender

## 概要

DiscordでメモをとってObsidianに自動で同期できるプラグインです。

**主な機能：**
- Discordのメッセージを自動でObsidianのmarkdownファイルに変換・保存
- URLの内容を自動でクリッピングして保存
- デスクトップ版Obsidian起動時またはコマンドパレットから実行

## 使用方法の流れ

1. **Discord環境の準備**
   - Obsidian連携用のDiscordサーバーを作成
   - 専用ボットを作成し、サーバーに招待
   - 連携用チャンネルを指定（チャンネルIDを使用）

2. **メッセージの処理**
   - Obsidianを起動するとDiscord APIでメッセージを取得
   - 通常のメッセージ → 日付別のmarkdownファイルに保存
   - コマンド（prefixから始まるメッセージ）→ 特殊処理を実行
   - 処理完了後、Discordに完了通知を送信

## ⚠️ 注意点

- **セキュリティ**: Discord APIを使用するため、機密情報の送信は避けてください
- **対応環境**: デスクトップ版Obsidianのみ対応

## セットアップ手順

### 1. Discordボットの作成

1. [Discord Developer Portal](https://discord.com/developers/applications)にアクセス
2. **New Application**ボタンから新規アプリケーションを作成
   ![image](https://d1fhrovvkiovx5.cloudfront.net/642c9b33b0d8250e770448b88d78e2c2.png)

3. **Bot設定**
   - 左ペインから**Bot**を選択
   - **Message Content Intent**を有効化
   ![image](https://d1fhrovvkiovx5.cloudfront.net/d284d81647f3dbf52a040cc7a6aa1362.png)
   - **トークンを保存**（⚠️重要: 取り扱い注意）

### 2. ボットをサーバーに招待

1. 左ペインの**OAuth2** → **OAuth2 URL Generator**へ移動
   ![image](https://d1fhrovvkiovx5.cloudfront.net/02355b8d6747734b75ae7b9799203132.png)

2. **Scopes**で`bot`を選択

3. **Bot Permissions**で以下を有効化：
   - View Channels
   - Send Messages
   - Read Message History
   - Add Reactions

4. 生成されたURLでボットを招待

### 3. チャンネルIDの取得

1. Discord設定 → 詳細設定 → **開発者モード**を有効化
2. 使用したいチャンネルを右クリック → **チャンネルIDをコピー**

### 4. プラグイン設定に必要な情報

以下の2つの情報をプラグイン設定で入力してください：
- **ボットトークン**
- **チャンネルID**

## コマンド一覧

設定可能なprefix（デフォルト: `!`）から始まるメッセージは特殊コマンドとして処理されます。

### `!url` - URL内容のクリッピング (調整中)

**使用例:**
```
!url https://www.example.com
```

**動作:**
- 指定URLの内容を自動取得
- Markdown形式で保存
- 保存先: 設定で指定したディレクトリ（デフォルト: **DiscordClippings**）

### 今後の予定

有用なコマンドがあれば随時追加予定です。

## 参考資料

本プラグインは以下のプロジェクトを参考に開発されました：
- [line_to_obsidian](https://github.com/onikun94/line_to_obsidian)
