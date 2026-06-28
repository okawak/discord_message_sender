import { Notice, type RequestUrlResponse, requestUrl } from "obsidian";
import { DiscordApiError, type DiscordRequestMethod } from "./discordApiError";
import type { DiscordMessage } from "./messages";

const DISCORD_API_BASE_URL = "https://discord.com/api/v10";
const RATE_LIMIT_STATUS_CODE = 429;
const MESSAGES_PER_REQUEST = 100;
const MAX_RETRIES = 3;

// Get message from Discord
export async function fetchMessages(
  botToken: string,
  channelId: string,
  after?: string,
): Promise<DiscordMessage[]> {
  const path = `/channels/${channelId}/messages?limit=${MESSAGES_PER_REQUEST}${
    after ? `&after=${after}` : ""
  }`;
  const res = await discordRequest(botToken, "GET", path);
  return JSON.parse(res.text);
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
    const res = await requestUrl({
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

    // Handle rate limiting
    if (res.status === RATE_LIMIT_STATUS_CODE) {
      if (i === MAX_RETRIES) {
        throw new DiscordApiError(res.status, method, path, res.text);
      }

      const wait = Number(res.headers["Retry-After"] ?? 1) * 1000 * (i + 1);
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
