English | [日本語](.github/README.ja.md)

# discord_message_sender

## Overview

This is a Obsidian plugin that allows you to take notes in Discord and automatically sync them to Obsidian.

**Key Features:**
- Automatically converts Discord messages into Obsidian Markdown files and saves them
- Automatically clips web page contents from URLs and saves them as Markdown by using `!url` command
- Can be triggered on Obsidian desktop startup or via the command palette

## Usage Flow

1. **Prepare Your Discord Environment**
   - Create a dedicated Discord server for Obsidian integration
   - Create a bot and invite it to your server
   - Specify the integration channel (using its channel ID)

2. **Message Processing**
   - When you launch Obsidian, the plugin fetches messages from Discord via the API
   - Regular messages → Saved as Markdown files, organized by date
   - Special commands (messages starting with the prefix) → Processed with custom handlers
   - After processing, a completion notification is sent to Discord

## ⚠️ Notes

- **Security:** Since this uses the Discord API, avoid sending sensitive or confidential information.
- **Supported Environment:** Only works with the desktop version of Obsidian.

## Setup Guide

### 1. Create a Discord Bot

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Click **New Application** to create a new app
   ![image](https://d1fhrovvkiovx5.cloudfront.net/642c9b33b0d8250e770448b88d78e2c2.png)
3. **Bot Settings**
   - Select **Bot** from the left menu
   - Enable **Message Content Intent**
     ![image](https://d1fhrovvkiovx5.cloudfront.net/d284d81647f3dbf52a040cc7a6aa1362.png)
   - **Save the bot token** (⚠️ Important: Keep it secure)

### 2. Invite the Bot to Your Server

1. Go to **OAuth2** → **OAuth2 URL Generator** in the left menu
   ![image](https://d1fhrovvkiovx5.cloudfront.net/02355b8d6747734b75ae7b9799203132.png)
2. Under **Scopes**, select `bot`
3. Under **Bot Permissions**, enable:
   - View Channels
   - Send Messages
   - Read Message History
   - Add Reactions
4. Use the generated URL to invite your bot

### 3. Get the Channel ID

1. In Discord settings, enable **Developer Mode** (in Advanced settings)
2. Right-click the channel you want to use → **Copy Channel ID**

### 4. Required Plugin Settings

Please enter the following information in the plugin settings:
- **Bot Token**
- **Channel ID**

## Command List

Messages starting with the configured prefix (default: `!`) are treated as special commands.

### `!url` - Web Page Clipping

**Example:**
```
!url https://www.example.com
```

**Behavior:**
- Fetches the contents of the specified URL
- Saves it as a Markdown file
- Save location: Directory specified in the settings (default: **DiscordClippings**)

### Roadmap

Additional useful commands will be added in future releases.

## References

This plugin was inspired by the following project(s):
- [line_to_obsidian](https://github.com/onikun94/line_to_obsidian)
