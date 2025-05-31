import { requestUrl } from "obsidian";

declare global {
  interface DiscordMsgSyncNS {
    fetchUrlContent?: (url: string) => Promise<string>;
  }
  interface Window {
    discordMsgSync?: DiscordMsgSyncNS;
  }
}

const fetchUrlContentImpl = async (url: string): Promise<string> => {
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

function getNamespace(): DiscordMsgSyncNS {
  if (!window.discordMsgSync) {
    window.discordMsgSync = {};
  }
  return window.discordMsgSync;
}

const ns = getNamespace();
if (!ns.fetchUrlContent) {
  ns.fetchUrlContent = fetchUrlContentImpl;
}
