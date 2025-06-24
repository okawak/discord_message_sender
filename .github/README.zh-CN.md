[English](../README.md) | [日本語](./README.ja.md) | 简体中文

# discord_message_sender

## 概要

这是一个可以在 Discord 记笔记并自动同步到 Obsidian 的插件。

**主要功能：**
- 自动将 Discord 消息转换为 Obsidian 的 Markdown 文件并保存
- 自动剪藏 URL 内容并保存为 Markdown
- 可在启动 Obsidian 桌面版时或通过命令面板执行

## 使用流程

1. **准备 Discord 环境**
   - 创建一个用于 Obsidian 集成的专用 Discord 服务器
   - 创建专用机器人并邀请到服务器
   - 指定要同步的频道（需要频道 ID）

2. **消息处理**
   - 启动 Obsidian 时，插件会通过 Discord API 获取消息
   - 普通消息 → 按日期保存为 Markdown 文件
   - 特殊命令（以前缀开头的消息）→ 进行特殊处理
   - 处理完成后，向 Discord 发送完成通知

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
   - 发送消息（Send Messages）
   - 读取消息历史（Read Message History）
   - 添加表情（Add Reactions）
4. 使用生成的 URL 邀请机器人

### 3. 获取频道 ID

1. Discord 设置 → 高级设置 → 启用 **开发者模式**
2. 右键点击需要的频道 → **复制频道 ID**

### 4. 插件设置所需信息

请在插件设置中输入以下两项信息：
- **机器人令牌**
- **频道 ID**

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
- 保存目录：在设置中指定（默认：**DiscordClippings**）

### 未来计划

将会持续添加更多实用的命令。

## 参考项目

本插件参考了以下项目开发：
- [line_to_obsidian](https://github.com/onikun94/line_to_obsidian)
