import { htmlToMarkdown, sanitizeHTMLToDom, stringifyYaml } from "obsidian";

const SAFE_PROTOCOLS = new Set(["https:", "mailto:", "tel:", "ftp:"]);
const EMBEDDED_RESOURCE_SELECTOR =
  "audio, embed, feImage, iframe, image, input, link, object, source, track, use, video";

export function convertHtml(url: string, html: string): string {
  const root = parseInertHtml(html);
  removeNonContentElements(root);
  const content =
    root.querySelector<HTMLElement>("article") ??
    root.querySelector<HTMLElement>("main") ??
    root;
  const title = extractTitle(content);
  const restoreImages = replaceImagesWithTokens(content, url);
  removeElements(content, EMBEDDED_RESOURCE_SELECTOR);
  resolveLinks(content, url);

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

function replaceImagesWithTokens(
  root: HTMLElement,
  baseUrl: string,
): (markdown: string) => string {
  const replacements: string[] = [];
  const tokenPrefix = `DMSIMAGE${crypto.randomUUID().replaceAll("-", "")}TOKEN`;

  for (const image of root.querySelectorAll<HTMLImageElement>("img")) {
    const source = image.getAttribute("src")?.trim();
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
  removeElements(root, "nav, footer");
}

function removeElements(root: ParentNode, selector: string): void {
  for (const element of root.querySelectorAll(selector)) {
    element.remove();
  }
}

function resolveLinks(root: ParentNode, baseUrl: string): void {
  for (const element of root.querySelectorAll<HTMLElement>("[href]")) {
    const value = element.getAttribute("href")?.trim();
    if (!value || value.startsWith("#")) {
      element.removeAttribute("href");
      continue;
    }

    const resolved = resolveUrl(value, baseUrl);
    if (resolved) element.setAttribute("href", resolved);
    else element.removeAttribute("href");
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
  return (
    root
      .querySelector<HTMLElement>("h1, h2, h3, h4, h5, h6")
      ?.textContent?.replace(/[\u200B\uFEFF]/g, "")
      .replace(/\s+/g, " ")
      .trim() || undefined
  );
}
