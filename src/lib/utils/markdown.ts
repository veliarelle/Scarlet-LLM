// Простой markdown-рендерер: порт из Scarlet LLM дизайн-прототипа,
// доработан для индентированных code-fences (часто встречаются внутри OL).

function escH(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function escAttr(s: string): string {
  return escH(s).replace(/'/g, "&#39;");
}

function sanitizeLang(s: string): string {
  const lang = s.trim().split(/\s+/)[0] ?? "";
  return /^[A-Za-z0-9_-]{1,40}$/.test(lang) ? lang : "text";
}

function safeHref(href: string): string | null {
  const trimmed = href.trim();
  if (!trimmed) return null;
  try {
    const url = new URL(trimmed);
    if (url.protocol === "http:" || url.protocol === "https:" || url.protocol === "mailto:") {
      return url.href;
    }
  } catch {
    return null;
  }
  return null;
}

function inlineMd(t: string): string {
  return t
    .replace(/\*\*\*(.+?)\*\*\*/g, "<strong><em>$1</em></strong>")
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    .replace(/\*(.+?)\*/g, "<em>$1</em>")
    .replace(/_([^_]+)_/g, "<em>$1</em>")
    .replace(/`([^`]+)`/g, '<code class="sl-icode">$1</code>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_m, label: string, href: string) => {
      const safe = safeHref(href);
      if (!safe) return label;
      return `<a href="${escAttr(safe)}" target="_blank" rel="noopener noreferrer nofollow">${label}</a>`;
    });
}

const FENCE_RE = /^(\s*)```(.*)$/;

export function renderMarkdown(text: string): string {
  const lines = text.split("\n");
  const out: string[] = [];
  let inCode = false;
  let lang = "";
  let codeIndent = "";
  let codeLines: string[] = [];
  let inUl = false;
  let inOl = false;

  const flushList = () => {
    if (inUl) {
      out.push("</ul>");
      inUl = false;
    }
    if (inOl) {
      out.push("</ol>");
      inOl = false;
    }
  };

  const closeCode = () => {
    const safeLang = sanitizeLang(lang);
    out.push(
      `<pre class="sl-codeblock"><code class="lang-${safeLang}">${escH(codeLines.join("\n"))}</code></pre>`
    );
    codeLines = [];
    inCode = false;
    lang = "";
    codeIndent = "";
  };

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const fenceMatch = raw.match(FENCE_RE);

    if (fenceMatch) {
      if (inCode) {
        flushList();
        closeCode();
      } else {
        flushList();
        codeIndent = fenceMatch[1];
        lang = fenceMatch[2].trim();
        inCode = true;
      }
      continue;
    }
    if (inCode) {
      // strip же тот же leading whitespace что и у открывающей fence
      const stripped =
        codeIndent && raw.startsWith(codeIndent) ? raw.slice(codeIndent.length) : raw;
      codeLines.push(stripped);
      continue;
    }

    const safe = escH(raw);

    if (/^#{1,3} /.test(safe)) {
      flushList();
      const m = safe.match(/^(#+)/);
      const lvl = m ? m[1].length : 1;
      const txt = inlineMd(safe.replace(/^#+\s/, ""));
      out.push(`<h${lvl} class="sl-md-h${lvl}">${txt}</h${lvl}>`);
      continue;
    }
    if (safe.startsWith("&gt; ")) {
      flushList();
      out.push(`<blockquote class="sl-md-bq">${inlineMd(safe.slice(5))}</blockquote>`);
      continue;
    }

    const ulMatch = safe.match(/^[-*]\s(.+)/);
    if (ulMatch) {
      if (inOl) {
        out.push("</ol>");
        inOl = false;
      }
      if (!inUl) {
        out.push('<ul class="sl-md-ul">');
        inUl = true;
      }
      out.push(`<li>${inlineMd(ulMatch[1])}</li>`);
      continue;
    }
    const olMatch = safe.match(/^\d+\.\s(.+)/);
    if (olMatch) {
      if (inUl) {
        out.push("</ul>");
        inUl = false;
      }
      if (!inOl) {
        out.push('<ol class="sl-md-ol">');
        inOl = true;
      }
      out.push(`<li>${inlineMd(olMatch[1])}</li>`);
      continue;
    }

    if (/^-{3,}$/.test(raw.trim())) {
      flushList();
      out.push('<hr class="sl-md-hr"/>');
      continue;
    }

    flushList();
    if (safe === "") {
      out.push('<div class="sl-md-br"></div>');
      continue;
    }
    out.push(`<p class="sl-md-p">${inlineMd(safe)}</p>`);
  }

  flushList();
  if (inCode && codeLines.length) {
    closeCode();
  }
  return out.join("");
}
