/**
 * Minimal, dependency-free Markdown/GFM subset renderer for the timeline.
 *
 * Security model: the input is HTML-escaped FIRST, so no original markup can
 * reach the DOM; only the whitelisted constructs below produce tags. Raw HTML
 * in agent output always renders as literal text.
 */

const ESCAPES: Record<string, string> = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, (character) => ESCAPES[character] ?? character);
}

function safeUrl(url: string): string {
  const trimmed = url.trim();
  if (/^(https?:\/\/|mailto:)/i.test(trimmed)) return trimmed;
  return "#";
}

function renderInline(text: string): string {
  let out = escapeHtml(text);
  out = out.replace(/`([^`]+)`/g, "<code>$1</code>");
  out = out.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  out = out.replace(/__([^_]+)__/g, "<strong>$1</strong>");
  out = out.replace(/(^|[\s(])\*([^*\s][^*]*)\*/g, "$1<em>$2</em>");
  out = out.replace(/(^|[\s(])_([^_\s][^_]*)_/g, "$1<em>$2</em>");
  out = out.replace(/~~([^~]+)~~/g, "<del>$1</del>");
  out = out.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (_match, label: string, url: string) =>
    `<a href="${safeUrl(url)}" target="_blank" rel="noopener noreferrer">${label}</a>`,
  );
  out = out.replace(/&lt;(https?:\/\/[^>]+)&gt;/gi, (_match, url: string) =>
    `<a href="${url}" target="_blank" rel="noopener noreferrer">${url}</a>`,
  );
  return out;
}

/** Render a Markdown/GFM subset to safe HTML (raw HTML is never allowed). */
export function renderMarkdown(source: string): string {
  const lines = source.replace(/\r\n/g, "\n").split("\n");
  const blocks: string[] = [];
  let listItems: string[] = [];
  let listOrdered = false;
  let paragraph: string[] = [];
  let codeFence: string[] | null = null;
  let codeLanguage = "";

  const flushParagraph = () => {
    if (paragraph.length > 0) {
      blocks.push(`<p>${paragraph.map(renderInline).join("<br/>")}</p>`);
      paragraph = [];
    }
  };
  const flushList = () => {
    if (listItems.length > 0) {
      const tag = listOrdered ? "ol" : "ul";
      blocks.push(`<${tag}>${listItems.map((item) => `<li>${renderInline(item)}</li>`).join("")}</${tag}>`);
      listItems = [];
    }
  };

  for (const line of lines) {
    const fence = line.match(/^\s*```\s*(\S*)\s*$/);
    if (fence) {
      if (codeFence === null) {
        flushParagraph();
        flushList();
        codeFence = [];
        codeLanguage = fence[1] ?? "";
      } else {
        blocks.push(
          `<pre data-lang="${escapeHtml(codeLanguage)}"><code>${codeFence.map(escapeHtml).join("\n")}</code></pre>`,
        );
        codeFence = null;
        codeLanguage = "";
      }
      continue;
    }
    if (codeFence !== null) {
      codeFence.push(line);
      continue;
    }

    const heading = line.match(/^(#{1,4})\s+(.*)$/);
    if (heading) {
      flushParagraph();
      flushList();
      const level = heading[1].length + 1; // demote: agent headings must not outrank the app
      blocks.push(`<h${level}>${renderInline(heading[2])}</h${level}>`);
      continue;
    }

    const bullet = line.match(/^\s*[-*+]\s+(.*)$/);
    if (bullet) {
      flushParagraph();
      if (listItems.length > 0 && !listOrdered) {
        listItems.push(bullet[1]);
      } else {
        flushList();
        listOrdered = false;
        listItems.push(bullet[1]);
      }
      continue;
    }
    const ordered = line.match(/^\s*\d+[.)]\s+(.*)$/);
    if (ordered) {
      flushParagraph();
      if (listItems.length > 0 && listOrdered) {
        listItems.push(ordered[1]);
      } else {
        flushList();
        listOrdered = true;
        listItems.push(ordered[1]);
      }
      continue;
    }

    const quote = line.match(/^\s*>\s?(.*)$/);
    if (quote) {
      flushParagraph();
      flushList();
      blocks.push(`<blockquote>${renderInline(quote[1])}</blockquote>`);
      continue;
    }

    if (line.trim() === "") {
      flushParagraph();
      flushList();
      continue;
    }
    flushList();
    paragraph.push(line);
  }
  if (codeFence !== null) {
    blocks.push(`<pre data-lang="${escapeHtml(codeLanguage)}"><code>${codeFence.map(escapeHtml).join("\n")}</code></pre>`);
  }
  flushParagraph();
  flushList();
  return blocks.join("");
}
