import { mock } from "bun:test";

let parsedInput = "";
let sanitizedInput = "";

mock.module("obsidian", () => ({
  sanitizeHTMLToDom(html: string) {
    sanitizedInput = html;
    return {};
  },
  htmlToMarkdown() {
    return sanitizedInput;
  },
  stringifyYaml() {
    return "source: https://qiita.com/example";
  },
}));

function createImage(source: string, alt: string) {
  let replacement: string | undefined;
  return {
    alt,
    getAttribute(name: string) {
      return name === "data-dms-src" ? source : null;
    },
    replaceWith(value: string) {
      replacement = value;
    },
    serialize() {
      return replacement ?? `<img data-dms-src="${source}">`;
    },
  };
}

const articleImage = createImage("//cdn.qiita.com/article.png", "Article");
const footerImage = createImage("//cdn.qiita.com/footer.png", "Footer");
let footerAttached = true;
const footer = { remove: () => (footerAttached = false) };
const body = {
  get innerHTML() {
    const footerHtml = footerAttached
      ? `<footer>${footerImage.serialize()}</footer>`
      : "";
    return `<p>Article</p>${articleImage.serialize()}${footerHtml}`;
  },
  get textContent() {
    return this.innerHTML;
  },
  querySelectorAll(selector: string) {
    if (selector === "nav, footer") return footerAttached ? [footer] : [];
    if (selector === "img") {
      return footerAttached ? [articleImage, footerImage] : [articleImage];
    }
    return [];
  },
  querySelector(selector: string) {
    if (selector === "article" && footerAttached) {
      return { innerHTML: "Related article", querySelectorAll: () => [] };
    }
    if (selector === "main") return body;
    return null;
  },
};
const parsedDocument = { body, querySelector: () => null };

const originalDOMParser = globalThis.DOMParser;
let markdown = "";
try {
  globalThis.DOMParser = class {
    parseFromString(html: string) {
      parsedInput = html;
      return parsedDocument;
    }
  } as unknown as typeof DOMParser;
  const { convertHtml } = await import("../src/htmlConversion");
  markdown = convertHtml(
    "https://qiita.com/example",
    '<main><img src="//cdn.qiita.com/article.png"><iframe src="/embed"></iframe></main>',
  );
} finally {
  globalThis.DOMParser = originalDOMParser;
}

if (parsedInput.includes(" src=") || !parsedInput.includes("data-dms-src=")) {
  throw new Error(`Loadable attributes reached DOMParser: ${parsedInput}`);
}
if (sanitizedInput !== "<p>Article</p>DMSIMAGETOKEN0END") {
  throw new Error(`Loadable elements reached the sanitizer: ${sanitizedInput}`);
}
if (!markdown.includes("![Article](<https://cdn.qiita.com/article.png>)")) {
  throw new Error(`Image URL was not restored in Markdown: ${markdown}`);
}

console.log(
  "HTML conversion masks resources before parsing and restores safe Markdown image URLs.",
);
