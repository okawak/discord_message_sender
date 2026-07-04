import type { DiscordMessage, ProcessedMessage } from "./messages";

export async function processDiscordMessageBatch(
  messages: readonly DiscordMessage[],
  parseMessage: (message: DiscordMessage) => Promise<ProcessedMessage>,
  saveMessages: (messages: readonly ProcessedMessage[]) => Promise<number>,
): Promise<number> {
  const processedMessages: ProcessedMessage[] = [];

  try {
    for (const message of messages) {
      if (message.author?.bot) {
        continue;
      }

      const processedMessage = await parseMessage(message);
      if (processedMessage.markdown) {
        processedMessages.push(processedMessage);
      }
    }
  } catch (error) {
    if (processedMessages.length > 0) {
      await saveMessages(processedMessages);
    }
    throw error;
  }

  return saveMessages(processedMessages);
}
