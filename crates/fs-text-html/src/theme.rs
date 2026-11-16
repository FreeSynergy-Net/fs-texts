//! The fixed style sheet of generated pages.
//!
//! Pattern: none (constants). No `@import`, no `url(...)`, no web font: system fonts only, so
//! a page loads nothing from outside. Light and dark follow the reader's setting; the focus
//! stays visible; no `style` attribute is needed anywhere.

/// The style sheet. The site builder writes it under a name made from its hash.
pub const CSS: &str = "\
:root{color-scheme:light dark;--bg:#fff;--fg:#1b1f24;--muted:#57606a;--line:#d0d7de;--link:#0b5cad;--soft:#f6f8fa;--accent:#0b5cad}
@media (prefers-color-scheme:dark){:root{--bg:#0f1318;--fg:#e6edf3;--muted:#9aa4af;--line:#30363d;--link:#79b8ff;--soft:#161b22;--accent:#79b8ff}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);font:1rem/1.6 system-ui,-apple-system,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif}
a{color:var(--link)}
a:focus-visible,summary:focus-visible{outline:3px solid var(--accent);outline-offset:2px}
.skip{position:absolute;left:-999px}
.skip:focus{left:.5rem;top:.5rem;background:var(--bg);padding:.5rem;z-index:10}
header,footer{padding:.75rem 1.25rem;border-bottom:1px solid var(--line)}
footer{border-top:1px solid var(--line);border-bottom:0;color:var(--muted);font-size:.9rem}
header{display:flex;flex-wrap:wrap;gap:1rem;align-items:center;justify-content:space-between}
.site{font-weight:700;margin:0}
.langs ul{display:flex;gap:.75rem;list-style:none;margin:0;padding:0}
.layout{display:flex;flex-wrap:wrap;gap:2rem;padding:1rem 1.25rem}
nav.side{flex:0 0 16rem;max-width:100%}
nav.side ul{list-style:none;margin:0;padding-left:1rem}
nav.side>ul{padding-left:0}
nav.side a[aria-current=page]{font-weight:700;color:var(--fg);text-decoration:none}
main{flex:1 1 30rem;min-width:0;max-width:52rem}
.crumbs ol{display:flex;flex-wrap:wrap;gap:.4rem;list-style:none;margin:0 0 1rem;padding:0;color:var(--muted);font-size:.9rem}
.crumbs li+li::before{content:'/';margin-right:.4rem}
h1,h2,h3,h4,h5,h6{line-height:1.25}
img{max-width:100%;height:auto}
pre{overflow:auto;background:var(--soft);padding:.75rem;border-radius:6px}
code{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:.92em}
table{border-collapse:collapse;display:block;overflow-x:auto;max-width:100%}
th,td{border:1px solid var(--line);padding:.35rem .6rem}
th{background:var(--soft)}
.al-l{text-align:left}.al-c{text-align:center}.al-r{text-align:right}
blockquote{margin:1rem 0;padding:.1rem 1rem;border-left:4px solid var(--line);color:var(--muted)}
.callout{margin:1rem 0;padding:.5rem 1rem;border:1px solid var(--line);border-left:4px solid var(--accent);background:var(--soft);border-radius:6px}
.callout-title{font-weight:700;margin:.25rem 0}
.callout-warning,.callout-danger{border-left-color:#bf8700}
li.task{list-style:none;margin-left:-1.25rem}
.footnotes{font-size:.9rem;color:var(--muted)}
.fn-back{text-decoration:none;margin-left:.25rem}
";
