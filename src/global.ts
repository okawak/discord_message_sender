import { requestUrl } from "obsidian";

declare global {
  interface DiscordMsgSyncNS {
    fetchUrlContent?: ((url: string) => Promise<string>) | undefined;
  }
  interface Window {
    discordMsgSync?: DiscordMsgSyncNS | undefined;
  }
}

window.discordMsgSync ??= {};
const namespace = window.discordMsgSync;
namespace.fetchUrlContent ??= async (url: string): Promise<string> => {
  if (new URL(url).protocol !== "https:") {
    throw new Error("Only HTTPS URLs are supported.");
  }
  const res = await requestUrl({
    url,
    method: "GET",
    headers: { "User-Agent": "Obsidian Discord Sender" },
  });
  return res.text;
};

export function cleanupGlobalNamespace(): void {
  delete window.discordMsgSync;
}
