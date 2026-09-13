import { mock } from "bun:test";
import assert from "node:assert/strict";

let sanitizedInput = "";
let serializedFrontmatter: Record<string, string> = {};
const fragment = {
  querySelector: () => ({ textContent: "Article heading" }),
};

// Only the Obsidian boundary is mocked. The HTML parser and serialization run
// against real (including malformed) input without any browser DOM available.
mock.module("obsidian", () => ({
  sanitizeHTMLToDom(html: string) {
    assert.doesNotMatch(
      html,
      /<(img|iframe|script|style|svg|math|template|input|link|object|audio|video)\b/i,
    );
    assert.doesNotMatch(
      html,
      /<[a-z][^<>]*\s(src|srcset|style|onerror|onload|background|ping|poster|data)=/i,
    );
    sanitizedInput = html;
    return fragment;
  },
  htmlToMarkdown(value: unknown) {
    assert.equal(
      value,
      fragment,
      "Only the sanitized fragment may reach the converter",
    );
    return sanitizedInput.replaceAll("<span>", "").replaceAll("</span>", "");
  },
  stringifyYaml(frontmatter: Record<string, string>) {
    serializedFrontmatter = frontmatter;
    return "source: https://qiita.com/example";
  },
}));

const { convertHtml } = await import("../src/htmlConversion");
const url = "https://qiita.com/example";

const markdown = convertHtml(
  url,
  `<!doctype html>
  <title>HTML &lt;img src=x&gt; guide</title>
  <footer><article><h1>Related</h1><img src="https://tracker.example/footer"></article></footer>
  <main><h1>Article heading</h1><p>Article</p>
    <span>DMSIMAGE</span><script>ignored</script><span>TOKEN0X0END</span>
    <a href="https://example.com/DMSIMAGETOKEN0X0END">Guide</a>
    <img src="//cdn.qiita.com/article.png" alt="Article">
    <img src="tel:x>)![p](https://tracker.example)" alt="Unsafe">
    <img src="#icon" alt="Icon">
    <img data-dms-src="https://tracker.example/pixel" alt="Reserved">
    <iframe src="/embed"></iframe>
  </main>`,
);
assert.deepEqual(serializedFrontmatter, {
  title: "Article heading",
  source: url,
});
assert.ok(markdown.includes("![Article](<https://cdn.qiita.com/article.png>)"));
assert.match(markdown, /Unsafe\s+Icon\s+Reserved/);
assert.equal(markdown.match(/DMSIMAGETOKEN0X0END/g)?.length, 2);
assert.doesNotMatch(
  markdown,
  /tracker\.example|Related|example#icon|HTML &lt;/,
);

const escaped = convertHtml(
  url,
  String.raw`<main>
  <img src="//cdn.qiita.com/backslash.png?q=\" alt="x\](https://attacker.invalid/p)![y">
  <img src="/a(b).png?q=&quot;&amp;x=1" alt="&lt;img src=x&gt; [safe]">
</main>`,
);
assert.ok(
  escaped.includes(
    String.raw`![x\\\](https://attacker.invalid/p)!\[y](<https://cdn.qiita.com/backslash.png?q=%5C>)`,
  ),
);
assert.ok(escaped.includes("https://qiita.com/a(b).png?q=%22&x=1"));
assert.ok(escaped.includes(String.raw`\[safe\]`));
assert.ok(escaped.includes(String.raw`\<img src=x\>`));

// Article selection must not depend on an earlier main element or on elements
// hidden inside a discarded subtree. Images outside the chosen section stay out.
const selected = convertHtml(
  url,
  `<nav><article>Discarded article<img src="/discarded.png"></article></nav>
  <main>Earlier main<img src="/main.png"></main>
  <article><p>Chosen article</p><img src="/chosen.png" alt="Cost $&"></article>
  <article>Later article<img src="/later.png"></article>`,
);
assert.match(selected, /Chosen article/);
assert.ok(selected.includes("![Cost $&](<https://qiita.com/chosen.png>)"));
assert.doesNotMatch(
  selected,
  /Discarded|Earlier|Later|discarded\.png|main\.png|later\.png/,
);

for (const html of [
  "<main>Chosen main</main><aside>Outside main</aside>",
  "<p>Chosen body</p><footer>Outside body</footer>",
]) {
  const fallback = convertHtml(url, html);
  assert.match(fallback, /Chosen/);
  assert.doesNotMatch(fallback, /Outside/);
}

convertHtml(
  url,
  `<main style="background:url(https://tracker.example/bg)">
  <p onload="steal()" style="background:url(https://tracker.example/p)">Keep</p>
  <a href="/guide?a=1&amp;b=2" ping="https://tracker.example/ping">Relative</a>
  <a href="javascript:steal()">Unsafe</a><a href="#part">Fragment</a>
  <a href="mailto:reader@example.com">Email</a>
  <img src="javascript:steal()" onerror="steal()" alt="No image">
  <img src="data:image/svg+xml,evil" alt="No data">
  <img src="file:///private/file" alt="No file">
  <style>@import 'https://tracker.example/style';</style>
  <script src="https://tracker.example/script">steal()</script>
  <svg><image href="https://tracker.example/svg"/></svg>
  <math><mtext><img src="https://tracker.example/math"></mtext></math>
  <template><article><img src="https://tracker.example/template"></article></template>
  <noscript><img src="https://tracker.example/noscript"></noscript>
  <input type="image" src="https://tracker.example/input">
  <video poster="https://tracker.example/poster"><source src="https://tracker.example/video"></video>
  <object data="https://tracker.example/object"></object>
  <table background="https://tracker.example/table"><tr><td colspan="2">Cell</td></tr></table>
</main>`,
);
assert.ok(
  sanitizedInput.includes(
    '<a href="https://qiita.com/guide?a=1&amp;b=2">Relative</a>',
  ),
);
assert.ok(
  sanitizedInput.includes('<a href="mailto:reader@example.com">Email</a>'),
);
assert.ok(sanitizedInput.includes('<td colspan="2">Cell</td>'));
assert.doesNotMatch(
  sanitizedInput,
  /tracker\.example|steal|javascript:|file:|data:image/,
);

// Exercise tree repair, foreign-content transitions, raw text, and template
// closing tags: these must never turn into resource loads at the DOM boundary.
for (const html of [
  '</template><img src="https://tracker.example/x" onerror="steal()"><p>End',
  '<table><p>Before<tr><td><img src="https://tracker.example/x"></table>After',
  '<math><mtext><table><mglyph><style><!--</style><img title="--><img src=x onerror=steal()>">',
  '<svg><foreignObject><img src="https://tracker.example/x"></foreignObject></svg><p>End',
  '<textarea>&lt;img src="https://tracker.example/x"&gt;</textarea>',
  '<xmp><img src="https://tracker.example/x"></xmp><p>End</p>',
  '<noembed><img src="https://tracker.example/x"></noembed><p>End</p>',
  '<plaintext><img src="https://tracker.example/x">',
]) {
  convertHtml(url, html);
}

console.log(
  "HTML conversion: real parsing, unsafe markup/resource removal, links, and image escaping passed.",
);
