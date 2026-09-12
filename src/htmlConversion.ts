import { htmlToMarkdown, sanitizeHTMLToDom, stringifyYaml } from "obsidian";

const SAFE_PROTOCOLS = new Set(["https:", "mailto:", "tel:", "ftp:"]);
const URL_ATTRIBUTES = ["href", "src"] as const;
const MASKED_ATTRIBUTE_PREFIX = "data-dms-";
const LOADABLE_ELEMENT_PATTERN =
  /<(?:audio|embed|feimage|iframe|image|img|input|link|object|source|track|use|video)\b(?:[^"'<>]|"[^"]*"|'[^']*')*>/gi;
const LOADABLE_ATTRIBUTE_PATTERN =
  /(\s)(src|srcdoc|srcset|poster|data|href|xlink:href)(\s*=)/gi;
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
  const document = new DOMParser().parseFromString(
    maskLoadableAttributes(html),
    "text/html",
  );
  const title = extractTitle(document);
  const body = document.body;
  removeNonContentElements(body);
  const content =
    body.querySelector<HTMLElement>("article") ??
    body.querySelector<HTMLElement>("main") ??
    body;
  resolveResourceUrls(content, url);
  const imageReplacements = replaceImagesWithTokens(content, url);

  const fragment = sanitizeHTMLToDom(content.innerHTML);
  const frontmatter = title ? { title, source: url } : { source: url };
  const yaml = stringifyYaml(frontmatter).trimEnd();
  let markdown = htmlToMarkdown(fragment);
  for (const [token, image] of imageReplacements) {
    markdown = markdown.replaceAll(token, image);
  }

  return `---\n${yaml}\n---\n\n${markdown.trim()}`;
}

function maskLoadableAttributes(html: string): string {
  return html.replace(LOADABLE_ELEMENT_PATTERN, (element) =>
    element.replace(
      LOADABLE_ATTRIBUTE_PATTERN,
      (_, whitespace: string, name: string, equals: string) =>
        `${whitespace}${MASKED_ATTRIBUTE_PREFIX}${name.toLowerCase()}${equals}`,
    ),
  );
}

function replaceImagesWithTokens(
  root: ParentNode,
  baseUrl: string,
): Array<readonly [string, string]> {
  const replacements: Array<readonly [string, string]> = [];
  let index = 0;

  for (const image of root.querySelectorAll<HTMLImageElement>("img")) {
    const source = image.getAttribute(`${MASKED_ATTRIBUTE_PREFIX}src`)?.trim();
    const alt = image.alt.replace(/\s+/g, " ").trim();
    const resolved = source ? resolveUrl(source, baseUrl) : undefined;
    if (!resolved) {
      image.replaceWith(alt);
      continue;
    }

    let token = `DMSIMAGETOKEN${index++}END`;
    while (root.textContent?.includes(token)) token += "X";
    image.replaceWith(token);
    replacements.push([
      token,
      `![${alt.replace(/([\\[\]])/g, "\\$1")}](<${resolved}>)`,
    ]);
  }

  return replacements;
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
