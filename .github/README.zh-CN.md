[English](../README.md) | [日本語](./README.ja.md) | 简体中文

# discord_message_sender

## 概要

这是一个可以在 Discord 记笔记并自动同步到 Obsidian 的插件。

**主要功能：**
- 自动将 Discord 消息转换为 Obsidian 的 Markdown 文件并保存
- 自动剪藏 URL 内容并保存为 Markdown
- 可同步多个 Discord 频道，并按频道保存到不同子文件夹
- 可将普通消息保存为单独、每日、每周或每月 Markdown 文件
- 可禁用或自定义同步完成后发送到 Discord 的通知消息
- 可在启动 Obsidian 桌面版时或通过命令面板执行

## 使用流程

1. **准备 Discord 环境**
   - 创建一个用于 Obsidian 集成的专用 Discord 服务器
   - 创建专用机器人并邀请到服务器
   - 指定一个或多个要同步的频道（需要频道 ID）

2. **消息处理**
   - 启动 Obsidian 时，插件会通过 Discord API 获取消息
   - 普通消息 → 按所选存储模式保存到对应频道的文件夹
   - 特殊命令（以前缀开头的消息）→ 进行特殊处理
   - 处理完成后，根据设置向 Discord 发送完成通知

## ⚠️ 注意事项

- **安全性**：由于使用了 Discord API，请避免发送敏感或机密信息
- **支持环境**：仅支持 Obsidian 桌面版

## 安装步骤

### 1. 创建 Discord 机器人

1. 访问 [Discord Developer Portal](https://discord.com/developers/applications)
2. 点击 **New Application** 创建新应用
   ![image](https://d1fhrovvkiovx5.cloudfront.net/642c9b33b0d8250e770448b88d78e2c2.png)
3. **机器人设置**
   - 在左侧菜单选择 **Bot**
   - 启用 **Message Content Intent**
     ![image](https://d1fhrovvkiovx5.cloudfront.net/d284d81647f3dbf52a040cc7a6aa1362.png)
   - **保存机器人令牌**（⚠️ 重要：请妥善保管）

### 2. 邀请机器人进入服务器

1. 在左侧菜单进入 **OAuth2** → **OAuth2 URL Generator**
   ![image](https://d1fhrovvkiovx5.cloudfront.net/02355b8d6747734b75ae7b9799203132.png)
2. 在 **Scopes** 中选择 `bot`
3. 在 **Bot Permissions** 中启用以下权限：
   - 查看频道（View Channels）
   - 发送消息（Send Messages，仅在启用同步通知时需要）
   - 读取消息历史（Read Message History）
   - 添加表情（Add Reactions）
4. 使用生成的 URL 邀请机器人

### 3. 获取频道 ID

1. Discord 设置 → 高级设置 → 启用 **开发者模式**
2. 右键点击每个需要同步的频道 → **复制频道 ID**

### 4. 插件设置所需信息

请在插件设置中输入以下信息：
- **机器人令牌**
- **频道**：添加每个 Discord 频道 ID。频道名称为可选项，会用作 Obsidian 中的子文件夹名称。请使用不会产生重复保存目录的名称。
- **消息存储**：可选择每条消息一个文件、每日、每周或每月日志。
- **显示作者名称**：在每日、每周和每月日志中包含 Discord 作者名称。
- **显示消息时间**：在每日、每周和每月日志中包含本地消息时间。
- **发送同步通知**：禁用后，插件不会向 Discord 发送同步完成消息。
- **通知模板**：可自定义同步后发送到 Discord 的消息。可用变量：`{count}`、`{channelName}`、`{channelId}`

默认情况下，普通消息会保存到 `DiscordLogs/<频道名称或ID>/`，URL 剪藏会保存到 `DiscordClippings/<频道名称或ID>/`。设置时会拒绝重复的保存目录名称；手动编辑设置文件造成的重复也会在同步前被检测。

频道名称不能包含 `\ / : * ? " < > | # ^ [ ]`，也不能使用 `.` 或 `..`。无效名称不会被保存。

## 消息存储

普通消息保存在 `DiscordLogs/<频道名称或ID>/` 中。可以选择以下模式：

- **每条消息一个文件**：`YYYYMMDD_HHMMSS_<消息ID>.md`
- **每日日志**：`YYYY-MM-DD.md`
- **每周日志**：`<ISO周年份>-W<周数>.md`，每周从星期一开始
- **每月日志**：`YYYY-MM.md`

日期、时间和单独文件名使用同步开始时计算机的本地时区。每日日志以日期作为一级标题。每周和每月日志以周期作为一级标题，并以每个日期作为二级标题。每条消息不会创建单独的标题。

关闭作者名称和消息时间时，聚合日志中的消息格式如下：

```markdown
<!-- discord-message-id: 1520291078606028900 -->
yes
```

同时启用这两个选项时，格式如下：

```markdown
<!-- discord-message-id: 1520291078606028900 -->
**Alice** · 21:34

yes
```

插件还会写入用于识别托管日志和日期的 HTML 注释。这些标记不会显示在 Obsidian 阅读视图中，并用于在同步重试或更改存储模式时防止重复消息。请勿删除或编辑这些标记。

现有设置会自动迁移为**每条消息一个文件**，并保留频道和同步游标。更改存储模式只影响新消息。现有文件不会被转换、移动或重命名，因此不同模式的文件可以安全共存。

无论选择哪种存储模式，URL 剪藏始终作为单独文件保存在 `DiscordClippings/<频道名称或ID>/` 中。

### Discord API 行为

插件使用 Discord API v10，每次最多请求 100 条消息。首次同步会导入最新的 100 条消息。后续同步只请求到达各频道已保存游标所需的页面，因此不会因每日、每周或每月日志而重新下载整个周期。新消息超过 100 条时会分页请求，并遵守 Discord 的速率限制响应。

## 命令列表

以配置的前缀（默认：`!`）开头的消息会被作为特殊命令处理。

### `!url` - 剪藏网页内容 (调整中)

**示例：**
```
!url https://www.example.com
```

**功能：**
- 自动获取指定 URL 的内容
- 保存为 Markdown 文件
- 保存目录：剪藏目录下的频道专属文件夹（默认：**DiscordClippings**）

### 未来计划

将会持续添加更多实用的命令。

## 参考项目

本插件参考了以下项目开发：
- [line_to_obsidian](https://github.com/onikun94/line_to_obsidian)
