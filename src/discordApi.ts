import { Notice, type RequestUrlResponse, requestUrl } from "obsidian";
import {
  discord_create_message_body as createDiscordMessageBody,
  type DiscordMessage,
  decode_discord_message as decodeDiscordMessage,
  decode_discord_messages as decodeDiscordMessages,
  discord_retry_decision,
  discord_messages_path as getChannelMessagesPath,
  discord_api_version as getDiscordApiVersion,
  discord_network_error_message as getNetworkErrorMessage,
  discord_rate_limit_notice as getRateLimitNotice,
  discord_reset_delay as getRateLimitResetDelay,
} from "../pkg/parse_message.js";
import { DiscordApiError, type DiscordRequestMethod } from "./wasmCore";

export interface DiscordMessagePage {
  messages: DiscordMessage[];
  nextRequestDelayMs: number;
}

export async function fetchMessages(
  botToken: string,
  channelId: string,
  before?: string,
): Promise<DiscordMessagePage> {
  const path = getChannelMessagesPath(channelId, before);
  const res = await discordRequest(botToken, "GET", path);
  return {
    messages: decodeDiscordMessages(res.text),
    nextRequestDelayMs: getRateLimitResetDelay(res.headers),
  };
}

// Post message to Discord
export async function postNotification(
  botToken: string,
  channelId: string,
  text: string,
): Promise<DiscordMessage> {
  const path = `/channels/${channelId}/messages`;
  const res = await discordRequest(
    botToken,
    "POST",
    path,
    createDiscordMessageBody(text),
  );
  return decodeDiscordMessage(res.text);
}

async function discordRequest(
  botToken: string,
  method: DiscordRequestMethod,
  path: string,
  body?: string,
): Promise<RequestUrlResponse> {
  for (let attempt = 0; ; attempt++) {
    let res: RequestUrlResponse;
    try {
      res = await requestUrl({
        url: `https://discord.com/api/v${getDiscordApiVersion()}${path}`,
        method,
        headers: {
          Authorization: `Bot ${botToken}`,
          "User-Agent": "DiscordBot (Discord Message Sender)",
          ...(body ? { "Content-Type": "application/json" } : {}),
        },
        ...(body ? { body } : {}),
        throw: false,
      });
    } catch (error) {
      const decision = discord_retry_decision(undefined, attempt, {}, "");
      if (decision.kind !== "retry") {
        throw new Error(getNetworkErrorMessage(method, path), {
          cause: error,
        });
      }
      await sleep(decision.delay);
      continue;
    }

    const decision = discord_retry_decision(
      res.status,
      attempt,
      res.headers,
      res.text,
    );
    if (decision.kind === "success") return res;
    if (decision.kind === "fail")
      throw new DiscordApiError(res.status, method, path, res.text);
    if (decision.rateLimited) new Notice(getRateLimitNotice(decision.delay));
    await sleep(decision.delay);
  }
}
