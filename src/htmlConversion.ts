import { htmlToMarkdown, sanitizeHTMLToDom, stringifyYaml } from "obsidian";

const SAFE_PROTOCOLS = new Set(["https:", "mailto:", "tel:", "ftp:"]);
const URL_ATTRIBUTES = ["href", "src"] as const;
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
  const imageSources = deactivateLoadableResources(content);
  resolveResourceUrls(content, url);
  const restoreImages = replaceImagesWithTokens(content, url, imageSources);

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

function deactivateLoadableResources(
  root: ParentNode,
): WeakMap<HTMLImageElement, string> {
  const imageSources = new WeakMap<HTMLImageElement, string>();
  for (const element of root.querySelectorAll<HTMLElement>(
    LOADABLE_ELEMENT_SELECTOR,
  )) {
    for (const attribute of LOADABLE_ATTRIBUTES) {
      const value = element.getAttribute(attribute);
      if (value === null) continue;
      if (element.localName === "img" && attribute === "src") {
        imageSources.set(element as HTMLImageElement, value);
      }
      element.removeAttribute(attribute);
    }
  }
  return imageSources;
}

function replaceImagesWithTokens(
  root: HTMLElement,
  baseUrl: string,
  imageSources: WeakMap<HTMLImageElement, string>,
): (markdown: string) => string {
  const replacements: string[] = [];
  const tokenPrefix = `DMSIMAGE${crypto.randomUUID().replaceAll("-", "")}TOKEN`;

  for (const image of root.querySelectorAll<HTMLImageElement>("img")) {
    const source = imageSources.get(image)?.trim();
    const alt = image.alt.replace(/\s+/g, " ").trim();
    const resolved =
      source && !source.startsWith("#")
        ? resolveUrl(source, baseUrl)
        : undefined;
    if (!resolved?.startsWith("https:")) {
      image.replaceWith(alt);
      continue;
    }

    const token = `${tokenPrefix}${replacements.length}END`;
    image.replaceWith(token);
    const escapedAlt = alt.replaceAll("\\", "\\\\").replace(/([[\]])/g, "\\$1");
    const escapedUrl = resolved.replaceAll("\\", "%5C");
    replacements.push(`![${escapedAlt}](<${escapedUrl}>)`);
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
  const titleElement = root.querySelector<HTMLElement>("title:not(svg *)");
  if (titleElement) {
    const decoder = titleElement.ownerDocument.createElement("textarea");
    decoder.innerHTML = titleElement.innerHTML;
    const title = normalizeTitle(decoder.textContent ?? undefined);
    if (title) return title;
  }

  for (const selector of TITLE_SELECTORS) {
    const element = root.querySelector<HTMLElement>(selector);
    const value =
      element?.getAttribute("content") ?? element?.textContent ?? undefined;
    const normalized = normalizeTitle(value);
    if (normalized) return normalized;
  }
  return undefined;
}

function normalizeTitle(value: string | undefined): string | undefined {
  return value
    ?.replace(/[\u200B\uFEFF]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}
