export type DiscordRequestMethod = "GET" | "POST";

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

export function createDiscordApiError(
  status: number,
  method: DiscordRequestMethod,
  path: string,
  responseText: string,
): DiscordApiError {
  return new DiscordApiError(
    getDiscordApiErrorMessage(status, method, path, responseText),
    status,
    method,
    path,
    responseText,
  );
}

export function getDiscordApiFailureNotice(error: DiscordApiError): string {
  if (error.status === 403) {
    const permission =
      error.method === "GET"
        ? "View Channel / Read Message History"
        : "Send Messages";
    return `missing Discord permission (${permission})`;
  }

  if (error.status === 401) {
    return "invalid Discord bot token";
  }

  if (error.status === 404) {
    return "Discord channel was not found";
  }

  return `Discord API returned ${error.status}`;
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
