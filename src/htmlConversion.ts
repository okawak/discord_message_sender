import { htmlToMarkdown, sanitizeHTMLToDom, stringifyYaml } from "obsidian";

const SAFE_PROTOCOLS = new Set(["https:", "mailto:", "tel:", "ftp:"]);
const URL_ATTRIBUTES = ["href", "src"] as const;
const MASKED_ATTRIBUTE_PREFIX = "data-dms-";
const LOADABLE_ELEMENT_SELECTOR =
  "audio, embed, feImage, iframe, image, img, input, link, object, source, track, use, video";
const LOADABLE_ATTRIBUTES = [
  "src",
  "srcdoc",
  "srcset",
  "poster",
  "data",
  "href",
  "xlink:href",
] as const;
const TITLE_SELECTORS = [
  "title:not(svg *)",
  'meta[name="title"]',
  'meta[property="og:title"]',
  'meta[name="twitter:title"]',
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
] as const;

export function convertHtml(url: string, html: string): string {
  const root = parseInertHtml(html);
  const title = extractTitle(root);
  removeNonContentElements(root);
  const content =
    root.querySelector<HTMLElement>("article") ??
    root.querySelector<HTMLElement>("main") ??
    root;
  maskLoadableAttributes(content);
  resolveResourceUrls(content, url);
  const restoreImages = replaceImagesWithTokens(content, url);

  const fragment = sanitizeHTMLToDom(content.innerHTML);
  const frontmatter = title ? { title, source: url } : { source: url };
  const yaml = stringifyYaml(frontmatter).trimEnd();
  const markdown = restoreImages(htmlToMarkdown(fragment)).trim();

  return `---\n${yaml}\n---\n\n${markdown}`;
}

function parseInertHtml(html: string): HTMLElement {
  const template = document.createElement("template");
  template.innerHTML = html;
  const root = template.content.ownerDocument.createElement("div");
  root.append(template.content);
  return root;
}

function maskLoadableAttributes(root: ParentNode): void {
  for (const element of root.querySelectorAll<HTMLElement>(
    LOADABLE_ELEMENT_SELECTOR,
  )) {
    for (const attribute of LOADABLE_ATTRIBUTES) {
      const value = element.getAttribute(attribute);
      if (value === null) continue;
      element.removeAttribute(attribute);
      element.setAttribute(
        `${MASKED_ATTRIBUTE_PREFIX}${attribute.replace(":", "-")}`,
        value,
      );
    }
  }
}

function replaceImagesWithTokens(
  root: HTMLElement,
  baseUrl: string,
): (markdown: string) => string {
  const replacements: string[] = [];
  const existingContent = `${root.innerHTML}\n${root.textContent ?? ""}`;
  let tokenPrefix = "DMSIMAGETOKEN";
  while (existingContent.includes(tokenPrefix)) tokenPrefix += "X";

  for (const image of root.querySelectorAll<HTMLImageElement>("img")) {
    const source = image.getAttribute(`${MASKED_ATTRIBUTE_PREFIX}src`)?.trim();
    const alt = image.alt.replace(/\s+/g, " ").trim();
    const resolved = source ? resolveUrl(source, baseUrl) : undefined;
    if (!resolved?.startsWith("https:")) {
      image.replaceWith(alt);
      continue;
    }

    const token = `${tokenPrefix}${replacements.length}END`;
    image.replaceWith(token);
    replacements.push(`![${alt.replace(/([\\[\]])/g, "\\$1")}](<${resolved}>)`);
  }

  if (replacements.length === 0) return (markdown) => markdown;
  const tokenPattern = new RegExp(`${tokenPrefix}(\\d+)END`, "g");
  return (markdown) =>
    markdown.replace(
      tokenPattern,
      (token, index: string) => replacements[Number(index)] ?? token,
    );
}

function removeNonContentElements(root: ParentNode): void {
  for (const element of root.querySelectorAll("nav, footer")) {
    element.remove();
  }
}

function resolveResourceUrls(root: ParentNode, baseUrl: string): void {
  for (const element of root.querySelectorAll<HTMLElement>("[href], [src]")) {
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

      const resolved = resolveUrl(value, baseUrl);
      if (!resolved) {
        element.removeAttribute(attribute);
        continue;
      }
      element.setAttribute(attribute, resolved);
    }
  }
}

function resolveUrl(value: string, baseUrl: string): string | undefined {
  try {
    const resolved = new URL(value, baseUrl);
    return SAFE_PROTOCOLS.has(resolved.protocol)
      ? resolved.toString()
      : undefined;
  } catch {
    return undefined;
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
