import { requestUrl } from "obsidian";

export async function fetchUrlContent(value: string): Promise<string> {
  let url: URL;
  try {
    url = new URL(value);
  } catch (error) {
    throw new Error("URL command requires a valid absolute URL.", {
      cause: error,
    });
  }

  if (url.protocol !== "https:") {
    throw new Error("Only HTTPS URLs are supported.");
  }

  try {
    const response = await requestUrl({
      url: url.toString(),
      method: "GET",
      headers: { "User-Agent": "Obsidian Discord Sender" },
    });
    return response.text;
  } catch (error) {
    throw new Error("Failed to fetch URL content.", { cause: error });
  }
}
