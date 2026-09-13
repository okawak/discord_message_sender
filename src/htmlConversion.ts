import { htmlToMarkdown, sanitizeHTMLToDom, stringifyYaml } from "obsidian";
import {
  type DefaultTreeAdapterTypes,
  html as htmlNames,
  parse,
  serialize,
  defaultTreeAdapter as tree,
} from "parse5";

type HtmlElement = DefaultTreeAdapterTypes.Element;

const SAFE_PROTOCOLS = new Set(["https:", "mailto:", "tel:", "ftp:"]);
const NON_CONTENT_TAGS = new Set([
  "audio",
  "embed",
  "footer",
  "head",
  "iframe",
  "input",
  "link",
  "nav",
  "noembed",
  "noframes",
  "noscript",
  "object",
  "plaintext",
  "script",
  "source",
  "style",
  "template",
  "track",
  "video",
  "xmp",
]);
// Keep only attributes used by the Markdown converter. Resource attributes,
// inline CSS, event handlers, and foreign namespaces never reach a browser DOM.
const MARKDOWN_ATTRIBUTES = new Set([
  "title",
  "class",
  "start",
  "colspan",
  "rowspan",
]);

export function convertHtml(url: string, html: string): string {
  const { content, images, tokenPrefix } = prepareContent(url, html);
  const fragment = sanitizeHTMLToDom(content);
  const title = fragment
    .querySelector("h1, h2, h3, h4, h5, h6")
    ?.textContent?.replace(/[\u200B\uFEFF]/g, "")
    .replace(/\s+/g, " ")
    .trim();
  const frontmatter = title ? { title, source: url } : { source: url };
  const yaml = stringifyYaml(frontmatter).trimEnd();
  const markdown = htmlToMarkdown(fragment)
    .replace(
      new RegExp(`${tokenPrefix}\\d+END`, "g"),
      (token) => images.get(token) ?? token,
    )
    .trim();

  return `---\n${yaml}\n---\n\n${markdown}`;
}

function prepareContent(baseUrl: string, html: string) {
  // parse5 creates plain objects. Remove resources before creating a browser DOM.
  const root = parse(html);
  const sections = new Map<string, HtmlElement>();
  const images = new Map<string, string>();
  const tokenPrefix = `DMSIMAGE${crypto.randomUUID().replaceAll("-", "")}TOKEN`;
  const pending = [...root.childNodes].reverse();
  while (pending.length > 0) {
    const node = pending.pop();
    if (!node || !tree.isElementNode(node)) continue;
    if (
      node.namespaceURI !== htmlNames.NS.HTML ||
      NON_CONTENT_TAGS.has(node.tagName)
    ) {
      tree.detachNode(node);
      continue;
    }
    if (
      ["article", "main", "body"].includes(node.tagName) &&
      !sections.has(node.tagName)
    ) {
      sections.set(node.tagName, node);
    }
    if (node.tagName === "img") {
      const alt = (getAttribute(node, "alt") ?? "").replace(/\s+/g, " ").trim();
      const source = resolveUrl(getAttribute(node, "src"), baseUrl);
      let text = alt;
      if (source?.startsWith("https:")) {
        text = `${tokenPrefix}${images.size}END`;
        const escapedAlt = alt.replace(/([\\[\]<>])/g, "\\$1");
        const escapedUrl = source.replaceAll("\\", "%5C");
        images.set(text, `![${escapedAlt}](<${escapedUrl}>)`);
      }
      if (node.parentNode) tree.insertTextBefore(node.parentNode, text, node);
      tree.detachNode(node);
      continue;
    }
    const href =
      node.tagName === "a"
        ? resolveUrl(getAttribute(node, "href"), baseUrl)
        : undefined;
    node.attrs = node.attrs.filter(
      (attr) => !attr.namespace && MARKDOWN_ATTRIBUTES.has(attr.name),
    );
    if (href) node.attrs.push({ name: "href", value: href });
    for (const child of [...node.childNodes].reverse()) pending.push(child);
  }
  const content = serialize(
    sections.get("article") ??
      sections.get("main") ??
      sections.get("body") ??
      root,
  );
  return { content, images, tokenPrefix };
}

function getAttribute(element: HtmlElement, name: string): string | undefined {
  return element.attrs.find(
    (attribute) => !attribute.namespace && attribute.name === name,
  )?.value;
}

function resolveUrl(
  value: string | undefined,
  baseUrl: string,
): string | undefined {
  const source = value?.trim();
  if (!source || source.startsWith("#")) return undefined;
  try {
    const resolved = new URL(source, baseUrl);
    return SAFE_PROTOCOLS.has(resolved.protocol)
      ? resolved.toString()
      : undefined;
  } catch {
    return undefined;
  }
}
