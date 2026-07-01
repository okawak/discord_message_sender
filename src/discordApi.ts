import { Notice, type RequestUrlResponse, requestUrl } from "obsidian";
import { DiscordApiError, type DiscordRequestMethod } from "./discordApiError";
import { getRateLimitDelay, getRateLimitResetDelay } from "./discordRateLimit";
import { DISCORD_API_VERSION, getChannelMessagesPath } from "./discordRoutes";
import type { DiscordMessage } from "./messages";

const DISCORD_API_BASE_URL = `https://discord.com/api/v${DISCORD_API_VERSION}`;
const RATE_LIMIT_STATUS_CODE = 429;
const MAX_RETRIES = 3;

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
  const messages: unknown = JSON.parse(res.text);
  if (!Array.isArray(messages)) {
    throw new TypeError("Discord API returned an invalid message list.");
  }
  return {
    messages: messages as DiscordMessage[],
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
    JSON.stringify({ content: text }),
  );
  return JSON.parse(res.text);
}

async function discordRequest(
  botToken: string,
  method: DiscordRequestMethod,
  path: string,
  body?: string,
): Promise<RequestUrlResponse> {
  for (let i = 0; i <= MAX_RETRIES; i++) {
    let res: RequestUrlResponse;
    try {
      res = await requestUrl({
        url: DISCORD_API_BASE_URL + path,
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
      if (i === MAX_RETRIES) {
        throw new Error(`Discord API ${method} ${path} request failed.`, {
          cause: error,
        });
      }
      await sleep(1000 * (i + 1));
      continue;
    }

    // Handle rate limiting
    if (res.status === RATE_LIMIT_STATUS_CODE) {
      if (i === MAX_RETRIES) {
        throw new DiscordApiError(res.status, method, path, res.text);
      }

      const wait = getRateLimitDelay(res.headers, res.text);
      new Notice(`Rate-limited. Retry after ${Math.ceil(wait / 1000)}s`);
      await sleep(wait);
      continue;
    }

    if (res.status >= 200 && res.status < 300) return res;

    const error = new DiscordApiError(res.status, method, path, res.text);

    if (res.status < 500 || i === MAX_RETRIES) {
      throw error;
    }

    await sleep(1000 * (i + 1));
  }

  throw new Error("Discord request: unrecoverable error");
}
