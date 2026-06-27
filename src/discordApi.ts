import { Notice, type RequestUrlResponse, requestUrl } from "obsidian";
import type { DiscordMessage } from "./settings";

const DISCORD_API_BASE_URL = "https://discord.com/api/v10";
const RATE_LIMIT_STATUS_CODE = 429;
const MESSAGES_PER_REQUEST = 100;
const MAX_RETRIES = 3;

type DiscordRequestMethod = "GET" | "POST";

export class DiscordApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly method: DiscordRequestMethod,
    readonly path: string,
    readonly responseText: string,
  ) {
    super(message);
    this.name = "DiscordApiError";
  }
}

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
        throw createDiscordApiError(res, method, path);
      }

      const wait = Number(res.headers["Retry-After"] ?? 1) * 1000 * (i + 1);
      new Notice(`Rate-limited. Retry after ${Math.ceil(wait / 1000)}s`);
      await sleep(wait);
      continue;
    }

    if (res.status >= 200 && res.status < 300) return res;

    const error = createDiscordApiError(res, method, path);
    console.error(error.message, res.text);

    if (res.status < 500 || i === MAX_RETRIES) {
      throw error;
    }

    await sleep(1000 * (i + 1));
  }

  throw new Error("Discord request: unrecoverable error");
}

function createDiscordApiError(
  res: RequestUrlResponse,
  method: DiscordRequestMethod,
  path: string,
): DiscordApiError {
  return new DiscordApiError(
    getDiscordApiErrorMessage(res.status, method, path, res.text),
    res.status,
    method,
    path,
    res.text,
  );
}

function getDiscordApiErrorMessage(
  status: number,
  method: DiscordRequestMethod,
  path: string,
  responseText: string,
): string {
  const detail = getDiscordErrorDetail(responseText);

  switch (status) {
    case 401:
      return `Discord API ${method} ${path} failed with 401 Unauthorized. Check the bot token.${detail}`;
    case 403:
      return `Discord API ${method} ${path} failed with 403 Forbidden. Check that the bot is invited to the server and has the required channel permissions.${detail}`;
    case 404:
      return `Discord API ${method} ${path} failed with 404 Not Found. Check the channel ID.${detail}`;
    default:
      return `Discord API ${method} ${path} failed with ${status}.${detail}`;
  }
}

function getDiscordErrorDetail(responseText: string): string {
  if (!responseText) {
    return "";
  }

  try {
    const payload: unknown = JSON.parse(responseText);
    if (isRecord(payload) && typeof payload.message === "string") {
      return ` Discord says: ${payload.message}`;
    }
  } catch {
    // Use the raw response text below.
  }

  return ` Discord response: ${responseText}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}
