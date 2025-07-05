use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static SUPPORTED_LANGUAGES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "rust",
        "python",
        "javascript",
        "typescript",
        "java",
        "csharp",
        "c",
        "cpp",
        "go",
        "php",
        "ruby",
        "swift",
        "kotlin",
        "dart",
        "scala",
        "clojure",
        "haskell",
        "ocaml",
        "fsharp",
        "erlang",
        "elixir",
        "lua",
        "perl",
        "r",
        "matlab",
        "julia",
        "nim",
        "zig",
        "crystal",
        "d",
        "pascal",
        "fortran",
        "cobol",
        "assembly",
        "bash",
        "shell",
        "sh",
        "zsh",
        "fish",
        "powershell",
        "batch",
        "html",
        "css",
        "scss",
        "sass",
        "less",
        "json",
        "yaml",
        "toml",
        "ini",
        "dockerfile",
        "sql",
        "graphql",
    ])
});

/// Represents a code block renderer that handles <pre>, <code>.
/// Also handles the element with attribute data-lang or class="code-frame".
pub struct CodeBlock;

impl CodeBlock {
    fn create_code_block(&self, content: &str, language: Option<String>) -> String {
        let capacity = content.len() + language.as_ref().map_or(0, |l| l.len()) + 10; // "```", newlines, etc.

        let mut result = String::with_capacity(capacity);

        result.push_str("```");
        if let Some(lang) = &language {
            result.push_str(lang);
        }
        result.push('\n');
        result.push_str(content);
        if !content.ends_with('\n') {
            result.push('\n');
        }
        result.push_str("```\n\n");
        result
    }

    fn find_code_content(dom: &Dom, id: NodeId) -> Result<String, ConvertError> {
        let Some(node) = dom.node(id) else {
            return Ok(String::new());
        };

        if let NodeData::Element { tag, .. } = &node.data {
            if tag.local.as_ref() == "code" {
                return Ok(Self::extract_text_content(dom, id));
            }
        }

        // recursively find code content in children
        for &child_id in dom.iter_children(id)? {
            if let Ok(content) = Self::find_code_content(dom, child_id) {
                if !content.trim().is_empty() {
                    return Ok(content);
                }
            }
        }

        Ok(String::new())
    }

    fn extract_text_content(dom: &Dom, id: NodeId) -> String {
        let Some(node) = dom.node(id) else {
            return String::new();
        };

        let mut result = String::new();
        for &child_id in &node.children {
            if let Some(child_node) = dom.node(child_id) {
                match &child_node.data {
                    NodeData::Text(text) => {
                        result.push_str(text);
                    }
                    NodeData::Element { .. } => {
                        result.push_str(&Self::extract_text_content(dom, child_id));
                    }
                    _ => {}
                }
            }
        }

        // remove trailing spaces and newlines
        result.trim_end().to_string()
    }

    /// Extracts the programming language from a code block.
    fn extract_language(&self, dom: &Dom, id: NodeId) -> Option<String> {
        let (_, attrs) = dom.get_element_data(id).ok()?;

        // data-lang attribute
        if let Some(lang) = attrs.get("data-lang") {
            return Some(lang.clone());
        }

        // class attribute
        if let Some(class) = attrs.get("class") {
            for class_name in class.split_whitespace() {
                // language-*, lang-*, highlight-* patterns
                if let Some(lang) = class_name.strip_prefix("language-") {
                    return Some(lang.to_string());
                }
                if let Some(lang) = class_name.strip_prefix("lang-") {
                    return Some(lang.to_string());
                }
                if let Some(lang) = class_name.strip_prefix("highlight-") {
                    return Some(lang.to_string());
                }

                // Check for standalone language names
                if self.is_valid_language(class_name) {
                    return Some(class_name.to_string());
                }
            }
        }

        // Check children recursively
        if let Ok(children) = dom.iter_children(id) {
            for &child_id in children {
                if let Some(lang) = self.extract_language(dom, child_id) {
                    return Some(lang);
                }
            }
        }
        None
    }

    fn is_valid_language(&self, name: &str) -> bool {
        SUPPORTED_LANGUAGES.contains(&name)
    }

    /// Render code block with preserved whitespace
    fn render_with_preserved_whitespace(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &mut Context,
    ) -> Result<String, ConvertError> {
        let old_preserve = ctx.preserve_whitespace;
        ctx.preserve_whitespace = true;
        let content = render_children(url, dom, id, ctx)?;
        ctx.preserve_whitespace = old_preserve;
        Ok(content)
    }

    /// check if code block has data-lang attribute
    fn has_code_lang_attribute(&self, attrs: &HashMap<String, String>) -> bool {
        attrs.contains_key("data-lang")
    }
    /// check code-frame class (<div class="code-frame">)
    fn is_code_frame(&self, attrs: &HashMap<String, String>) -> bool {
        attrs
            .get("class")
            .is_some_and(|class| class.contains("code-frame"))
    }
}

impl Renderer for CodeBlock {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return false;
        };

        let NodeData::Element { tag, attrs, .. } = &node.data else {
            return false;
        };

        match tag.local.as_ref() {
            "pre" => true,
            "code" => {
                // <code> in <pre> should not be rendered as inline code
                if let Ok(Some(parent_id)) = dom.get_parent(id) {
                    if let Ok((parent_tag, _)) = dom.get_element_data(parent_id) {
                        return parent_tag.local.as_ref() != "pre";
                    }
                }
                true
            }
            _ => self.has_code_lang_attribute(attrs) || self.is_code_frame(attrs),
        }
    }

    fn render(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &mut Context,
    ) -> Result<String, ConvertError> {
        let (tag, attrs) = dom.get_element_data(id)?;

        // process elements that have data-lang attribute and code-frame class
        if self.is_code_frame(attrs) || self.has_code_lang_attribute(attrs) {
            let language = self.extract_language(dom, id);
            let code_content = Self::find_code_content(dom, id)?;
            return Ok(self.create_code_block(&code_content, language));
        }

        match tag.local.as_ref() {
            "pre" => {
                let language = self.extract_language(dom, id);
                let content = self.render_with_preserved_whitespace(url, dom, id, ctx)?;
                Ok(self.create_code_block(&content, language))
            }
            "code" => {
                // inline code
                let content = self.render_with_preserved_whitespace(url, dom, id, ctx)?;
                Ok(format!("`{content}`"))
            }
            _ => render_children(url, dom, id, ctx),
        }
    }
}

pub static CODE_BLOCK: CodeBlock = CodeBlock;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::renderers;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// inline code test
    #[rstest]
    #[case(r#"<code class="language-rust">code</code>"#, "`code`")]
    #[case(r#"<code class="lang-python">code</code>"#, "`code`")]
    #[case(r#"<code class="highlight-javascript">code</code>"#, "`code`")]
    #[case(r#"<code class="rust">code</code>"#, "`code`")]
    #[case(r#"<code class="csharp">code</code>"#, "`code`")]
    #[case(r#"<code class="unknown-language">code</code>"#, "`code`")]
    #[case(r#"<code class="not-a-language">code</code>"#, "`code`")]
    #[case(r#"<code>code</code>"#, "`code`")]
    fn test_inline_code_elements(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render inline code");
        assert_eq!(result, expected);
    }

    /// code block tests
    #[rstest]
    #[case(r#"<pre><code>simple code</code></pre>"#, "```\nsimple code\n```\n\n")]
    #[case(
        r#"<pre><code class="language-rust">fn main() {}</code></pre>"#,
        "```rust\nfn main() {}\n```\n\n"
    )]
    #[case(
        r#"<pre><code class="lang-python">print("hello")</code></pre>"#,
        "```python\nprint(\"hello\")\n```\n\n"
    )]
    #[case(
        r#"<pre><code class="highlight-javascript">console.log();</code></pre>"#,
        "```javascript\nconsole.log();\n```\n\n"
    )]
    #[case(
        r#"<pre><code class="rust">let x = 42;</code></pre>"#,
        "```rust\nlet x = 42;\n```\n\n"
    )]
    #[case(
        r#"<pre><code class="csharp">var name = "test";</code></pre>"#,
        "```csharp\nvar name = \"test\";\n```\n\n"
    )]
    #[case(
        r#"<pre><code class="unknown-language">some code</code></pre>"#,
        "```\nsome code\n```\n\n"
    )]
    fn test_pre_code_blocks(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render code block");
        assert_eq!(result, expected);
    }

    /// multiple classes and language extraction
    #[rstest]
    #[case(
        r#"<pre><code class="hljs language-rust syntax-highlighting">fn main() {}</code></pre>"#,
        "```rust\nfn main() {}\n```\n\n"
    )]
    #[case(
        r#"<pre><code class="prettyprint lang-python linenums">print("hello")</code></pre>"#,
        "```python\nprint(\"hello\")\n```\n\n"
    )]
    #[case(
        r#"<pre><code class="highlight highlight-javascript syntax">console.log();</code></pre>"#,
        "```javascript\nconsole.log();\n```\n\n"
    )]
    #[case(
        r#"<pre><code class="code-block typescript syntax-ts">type T = string;</code></pre>"#,
        "```typescript\ntype T = string;\n```\n\n"
    )]
    #[case(
        r#"<pre><code class="styled-code syntax-highlighted">no language</code></pre>"#,
        "```\nno language\n```\n\n"
    )]
    fn test_multiple_classes_language_extraction(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render code block");
        assert_eq!(result, expected);
    }

    /// data-lang attribute tests
    #[rstest]
    #[case(
        r#"<div data-lang="typescript"><pre><code>type T = string;</code></pre></div>"#,
        "```typescript\ntype T = string;\n```\n\n"
    )]
    #[case(
        r#"<section data-lang="go"><pre><code>fmt.Println()</code></pre></section>"#,
        "```go\nfmt.Println()\n```\n\n"
    )]
    #[case(
        r#"<article data-lang="java"><pre><code>System.out.println();</code></pre></article>"#,
        "```java\nSystem.out.println();\n```\n\n"
    )]
    fn test_data_lang_attribute(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render code element");
        assert_eq!(result, expected);
    }

    /// complex nested code frame test (code-frame with multiple elements)
    #[rstest]
    #[case(
        r#"<div class="code-frame" data-lang="csharp" data-sourcepos="5:1-7:3">
            <div class="code-copy">
                <div class="code-copy__message" style="display: none;">Copied!</div>
                <button class="code-copy__button" style="display: none;">
                    <span class="fa fa-fw fa-clipboard"></span>
                </button>
            </div>
            <div class="highlight">
                <pre><code><span class="kt">var</span> <span class="n">name</span> <span class="p">=</span> <span class="n">person</span><span class="p">?.</span><span class="n">name</span><span class="p">;</span></code></pre>
            </div>
        </div>"#,
        "```csharp\nvar name = person?.name;\n```\n\n"
    )]
    fn test_complex_nested_code_frame(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render complex code frame");
        assert_eq!(result, expected);
    }

    /// preserving multiline code blocks
    #[rstest]
    #[case(
        r#"<pre><code>line1
line2
    indented</code></pre>"#,
        "```\nline1\nline2\n    indented\n```\n\n"
    )]
    #[case(
        r#"<pre><code class="language-python">def hello():
    print("Hello, World!")
    return True</code></pre>"#,
        "```python\ndef hello():\n    print(\"Hello, World!\")\n    return True\n```\n\n"
    )]
    fn test_multiline_preservation(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render code element");
        assert_eq!(result, expected);
    }

    /// empty code blocks
    #[rstest]
    #[case(r#"<pre><code></code></pre>"#, "```\n\n```\n\n")]
    #[case(r#"<code></code>"#, "``")]
    #[case(
        r#"<div class="code-frame"><pre><code></code></pre></div>"#,
        "```\n\n```\n\n"
    )]
    fn test_empty_code(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render code element");
        assert_eq!(result, expected);
    }

    /// invalid language names
    #[rstest]
    #[case("rust", true)]
    #[case("python", true)]
    #[case("javascript", true)]
    #[case("typescript", true)]
    #[case("csharp", true)]
    #[case("unknown-lang", false)]
    #[case("not-a-language", false)]
    #[case("", false)]
    fn test_is_valid_language(#[case] lang: &str, #[case] expected: bool) {
        let code_block = CodeBlock;
        assert_eq!(code_block.is_valid_language(lang), expected);
    }
}
