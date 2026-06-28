import type { DiscordChannelSettings } from "./settings";

interface NotificationTemplateContext {
  channel: DiscordChannelSettings;
  count: number;
}

export function renderNotificationTemplate(
  template: string,
  context: NotificationTemplateContext,
): string {
  const values: Record<string, string> = {
    count: context.count.toString(),
    channelId: context.channel.id,
    channelName: context.channel.name || context.channel.id,
  };

  return template.replace(/\{(count|channelId|channelName)\}/g, (_, key) => {
    return values[key] ?? "";
  });
}
