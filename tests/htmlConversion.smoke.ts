import { mock } from "bun:test";

let sanitizedInput = "";

mock.module("obsidian", () => ({
  sanitizeHTMLToDom(html: string) {
    sanitizedInput = html;
    return {};
  },
  htmlToMarkdown() {
    return "Converted article";
  },
  stringifyYaml() {
    return "source: https://qiita.com/example";
  },
}));

function createResource(value: string) {
  let src: string | undefined = value;
  return {
    getAttribute(name: string) {
      return name === "src" ? (src ?? null) : null;
    },
    setAttribute(name: string, next: string) {
      if (name === "src") src = next;
    },
    removeAttribute(name: string) {
      if (name === "src") src = undefined;
    },
    serialize() {
      return src ? `<img src="${src}">` : "<img>";
    },
  };
}

const articleImage = createResource("//cdn.qiita.com/article.png");
const footerImage = createResource("//cdn.qiita.com/footer.png");
let footerAttached = true;
const footer = { remove: () => (footerAttached = false) };
const body = {
  get innerHTML() {
    const footerHtml = footerAttached
      ? `<footer>${footerImage.serialize()}</footer>`
      : "";
    return `<p>Article</p>${articleImage.serialize()}${footerHtml}`;
  },
  querySelectorAll(selector: string) {
    if (selector === "nav, footer") return footerAttached ? [footer] : [];
    if (selector === "[href], [src]") {
      return footerAttached ? [articleImage, footerImage] : [articleImage];
    }
    return [];
  },
};
const parsedDocument = {
  body,
  querySelector() {
    return null;
  },
};

const originalDOMParser = globalThis.DOMParser;
try {
  globalThis.DOMParser = class {
    parseFromString() {
      return parsedDocument;
    }
  } as unknown as typeof DOMParser;
  const { convertHtml } = await import("../src/htmlConversion");
  convertHtml("https://qiita.com/example", "<html />");
} finally {
  globalThis.DOMParser = originalDOMParser;
}

const expected = '<p>Article</p><img src="https://cdn.qiita.com/article.png">';
if (sanitizedInput !== expected) {
  throw new Error(`Unexpected HTML passed to sanitizer: ${sanitizedInput}`);
}

console.log(
  "HTML conversion removes non-content and resolves resource URLs before sanitizing.",
);
