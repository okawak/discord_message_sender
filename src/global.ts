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
  try {
    const res = await requestUrl({
      url,
      method: "GET",
      headers: { "User-Agent": "Obsidian Discord Sender" },
    });
    return res.text;
  } catch (err) {
    console.error("fetchUrlContent error:", err);
    return `<!-- Failed to fetch ${url}: ${err} -->`;
  }
};

export function cleanupGlobalNamespace(): void {
  delete window.discordMsgSync;
}
