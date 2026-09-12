import { htmlToMarkdown, sanitizeHTMLToDom, stringifyYaml } from "obsidian";

const SAFE_PROTOCOLS = new Set(["https:", "mailto:", "tel:", "ftp:"]);
const URL_ATTRIBUTES = ["href", "src"] as const;
const TITLE_SELECTORS = [
  "head > title",
  'head meta[name="title"]',
  'head meta[property="og:title"]',
  'head meta[name="twitter:title"]',
  "body h1",
  "body h2",
  "body h3",
  "body h4",
  "body h5",
  "body h6",
] as const;

export function convertHtml(url: string, html: string): string {
  const title = extractTitle(
    new DOMParser().parseFromString(html, "text/html"),
  );
  const fragment = sanitizeHTMLToDom(html);
  removeNonContentElements(fragment);
  resolveResourceUrls(fragment, url);

  const content =
    fragment.querySelector<HTMLElement>("article") ??
    fragment.querySelector<HTMLElement>("main") ??
    fragment.querySelector<HTMLElement>("body") ??
    fragment;
  const frontmatter = title ? { title, source: url } : { source: url };
  const yaml = stringifyYaml(frontmatter).trimEnd();
  const markdown = htmlToMarkdown(content).trim();

  return `---\n${yaml}\n---\n\n${markdown}`;
}

function removeNonContentElements(fragment: DocumentFragment): void {
  for (const element of fragment.querySelectorAll("nav, footer")) {
    element.remove();
  }
}

function resolveResourceUrls(
  fragment: DocumentFragment,
  baseUrl: string,
): void {
  for (const element of fragment.querySelectorAll<HTMLElement>(
    "[href], [src]",
  )) {
    for (const attribute of URL_ATTRIBUTES) {
      const value = element.getAttribute(attribute)?.trim();
      if (value === undefined) continue;
      if (!value) {
        element.removeAttribute(attribute);
        continue;
      }

      if (value.startsWith("#")) {
        element.removeAttribute(attribute);
        continue;
      }

      try {
        const resolved = new URL(value, baseUrl);
        if (!SAFE_PROTOCOLS.has(resolved.protocol)) {
          element.removeAttribute(attribute);
          continue;
        }
        element.setAttribute(attribute, resolved.toString());
      } catch {
        element.removeAttribute(attribute);
      }
    }
  }
}

function extractTitle(root: ParentNode): string | undefined {
  for (const selector of TITLE_SELECTORS) {
    const element = root.querySelector<HTMLElement>(selector);
    const value =
      element?.getAttribute("content") ?? element?.textContent ?? undefined;
    const normalized = value
      ?.replace(/[\u200B\uFEFF]/g, "")
      .replace(/\s+/g, " ")
      .trim();
    if (normalized) return normalized;
  }
  return undefined;
}
