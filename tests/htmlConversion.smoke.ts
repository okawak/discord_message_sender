import { mock } from "bun:test";

let parsedInput = "";
let sanitizedInput = "";
let serializedFrontmatter: Record<string, string> = {};

mock.module("obsidian", () => ({
  sanitizeHTMLToDom(html: string) {
    sanitizedInput = html.replace("<script>ignored</script>", "");
    return {};
  },
  htmlToMarkdown() {
    return sanitizedInput.replaceAll("<span>", "").replaceAll("</span>", "");
  },
  stringifyYaml(frontmatter: Record<string, string>) {
    serializedFrontmatter = frontmatter;
    return "source: https://qiita.com/example";
  },
}));

function createResource(
  tag: "a" | "iframe" | "img",
  initialAttributes: Record<string, string>,
  alt = "",
) {
  const attributes = new Map(Object.entries(initialAttributes));
  let replacement: string | undefined;
  return {
    alt,
    localName: tag,
    getAttribute(name: string) {
      return attributes.get(name) ?? null;
    },
    setAttribute(name: string, value: string) {
      attributes.set(name, value);
    },
    removeAttribute(name: string) {
      attributes.delete(name);
    },
    replaceWith(value: string) {
      replacement = value;
    },
    serialize() {
      if (replacement !== undefined) return replacement;
      const serializedAttributes = [...attributes]
        .map(([name, value]) => ` ${name}="${value}"`)
        .join("");
      if (tag === "a") return `<a${serializedAttributes}>Guide</a>`;
      if (tag === "iframe") return `<iframe${serializedAttributes}></iframe>`;
      return `<img${serializedAttributes}>`;
    },
  };
}

const articleImage = createResource(
  "img",
  { src: "//cdn.qiita.com/article.png" },
  "Article",
);
const footerImage = createResource(
  "img",
  { src: "//cdn.qiita.com/footer.png" },
  "Footer",
);
const unsafeImage = createResource(
  "img",
  { src: "tel:x>)![p](https://tracker.example)" },
  "Unsafe",
);
const backslashImage = createResource(
  "img",
  { src: "//cdn.qiita.com/backslash.png?q=\\" },
  String.raw`x\](https://attacker.invalid/p)![y`,
);
const fragmentImage = createResource("img", { src: "#icon" }, "Icon");
const reservedImage = createResource(
  "img",
  { "data-dms-src": "https://tracker.example/pixel" },
  "Reserved",
);
const iframe = createResource("iframe", { src: "/embed" });
const anchor = createResource("a", {
  href: "https://example.com/DMSIMAGETOKEN0X0END",
});
let footerAttached = true;
const footer = { remove: () => (footerAttached = false) };
const heading = {
  getAttribute: () => null,
  textContent: "Article heading",
};
const root = {
  append() {},
  get innerHTML() {
    const footerHtml = footerAttached
      ? `<footer><article>${footerImage.serialize()}</article></footer>`
      : "";
    return `<p>Article</p><span>DMSIMAGE</span><script>ignored</script><span>TOKEN0X0END</span>${anchor.serialize()}${articleImage.serialize()}${unsafeImage.serialize()}${backslashImage.serialize()}${fragmentImage.serialize()}${reservedImage.serialize()}${iframe.serialize()}${footerHtml}`;
  },
  get textContent() {
    return "HTML <img src=x> guide Article Guide";
  },
  querySelectorAll(selector: string) {
    if (selector === "nav, footer") return footerAttached ? [footer] : [];
    if (selector.includes("iframe") && selector.includes("video")) {
      return footerAttached
        ? [
            articleImage,
            footerImage,
            unsafeImage,
            backslashImage,
            fragmentImage,
            reservedImage,
            iframe,
          ]
        : [
            articleImage,
            unsafeImage,
            backslashImage,
            fragmentImage,
            reservedImage,
            iframe,
          ];
    }
    if (selector === "[href], [src]") {
      return [
        articleImage,
        unsafeImage,
        backslashImage,
        fragmentImage,
        reservedImage,
        iframe,
        anchor,
      ].filter(
        (element) =>
          element.getAttribute("href") !== null ||
          element.getAttribute("src") !== null,
      );
    }
    if (selector === "img") {
      return [
        articleImage,
        unsafeImage,
        backslashImage,
        fragmentImage,
        reservedImage,
      ];
    }
    return [];
  },
  querySelector(selector: string) {
    if (selector === "article" && footerAttached) {
      return { innerHTML: "Related article", querySelectorAll: () => [] };
    }
    if (selector === "main") return root;
    if (selector === "h1") return heading;
    return null;
  },
};
const templateContent = {
  ownerDocument: {
    createElement() {
      return root;
    },
  },
};
const template = {
  content: templateContent,
  set innerHTML(html: string) {
    parsedInput = html;
  },
};
const fakeDocument = {
  createElement() {
    return template;
  },
};

const rawHtml =
  '<title>HTML <img src=x> guide</title><main><img src="//cdn.qiita.com/article.png"><iframe src="/embed"></iframe></main>';
const originalDocument = globalThis.document;
let markdown = "";
try {
  Object.defineProperty(globalThis, "document", {
    configurable: true,
    value: fakeDocument,
  });
  const { convertHtml } = await import("../src/htmlConversion");
  markdown = convertHtml("https://qiita.com/example", rawHtml);
} finally {
  if (originalDocument === undefined) {
    Reflect.deleteProperty(globalThis, "document");
  } else {
    Object.defineProperty(globalThis, "document", {
      configurable: true,
      value: originalDocument,
    });
  }
}

if (parsedInput !== rawHtml) {
  throw new Error(
    `HTML text was modified before inert parsing: ${parsedInput}`,
  );
}
if (
  articleImage.getAttribute("src") !== null ||
  iframe.getAttribute("src") !== null ||
  sanitizedInput.includes(" src=")
) {
  throw new Error(
    `Loadable attributes reached the sanitizer: ${sanitizedInput}`,
  );
}
if (serializedFrontmatter.title !== "Article heading") {
  throw new Error(
    `Content heading was not used as the title: ${serializedFrontmatter.title}`,
  );
}
if (
  !markdown.includes("![Article](<https://cdn.qiita.com/article.png>)") ||
  !markdown.includes(
    String.raw`![x\\\](https://attacker.invalid/p)!\[y](<https://cdn.qiita.com/backslash.png?q=%5C>)`,
  ) ||
  !markdown.includes("https://example.com/DMSIMAGETOKEN0X0END") ||
  markdown.match(/DMSIMAGETOKEN0X0END/g)?.length !== 2 ||
  markdown.includes("https://qiita.com/example#icon") ||
  markdown.includes("tracker.example")
) {
  throw new Error(`Image restoration damaged Markdown: ${markdown}`);
}
if (sanitizedInput.includes("Footer")) {
  throw new Error(`Footer reached the sanitizer: ${sanitizedInput}`);
}

console.log(
  "HTML conversion parses inertly, masks resource loads, and restores collision-free image Markdown.",
);
