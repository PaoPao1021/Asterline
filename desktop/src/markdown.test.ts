import { describe, expect, it } from "vitest";
import { renderMarkdown } from "./markdown";

describe("renderMarkdown", () => {
  it("never lets raw HTML through", () => {
    const html = renderMarkdown("<img src=x onerror=alert(1)> **bold**");
    expect(html).not.toContain("<img");
    expect(html).toContain("&lt;img");
    expect(html).toContain("<strong>bold</strong>");
  });

  it("renders fenced code blocks and inline code", () => {
    const html = renderMarkdown("use `cargo test` like:\n```\nfn main() {}\n```");
    expect(html).toContain("<pre");
    expect(html).toContain("<code>fn main() {}</code>");
    expect(html).toContain("<code>cargo test</code>");
  });

  it("renders headings lists and blockquotes", () => {
    const html = renderMarkdown("## Plan\n- one\n- two\n> quoted");
    expect(html).toContain("<h3>Plan</h3>");
    expect(html).toContain("<ul><li>one</li><li>two</li></ul>");
    expect(html).toContain("<blockquote>quoted</blockquote>");
  });

  it("only allows safe link protocols", () => {
    const html = renderMarkdown("[click](javascript:alert(1)) [ok](https://example.com)");
    expect(html).not.toContain("javascript:");
    expect(html).toContain('href="https://example.com"');
    expect(html).toContain('rel="noopener noreferrer"');
  });

  it("renders bold italic and strikethrough", () => {
    const html = renderMarkdown("**bold** *it* ~~gone~~");
    expect(html).toContain("<strong>bold</strong>");
    expect(html).toContain("<em>it</em>");
    expect(html).toContain("<del>gone</del>");
  });
});
