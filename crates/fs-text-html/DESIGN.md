# fs-text-html - Design

`fs-text-html` writes the text model of [`fs-text`](../fs-text) as HTML and builds the page around it. It is the first half of the
website export (stage 1, plan `plan-website-stufe1-2.md` part A); the site builder `fs-text-site` (folder in, output folder out) is the second.
It depends only on `fs-text` (public types; `fs-text` is not changed), has no GUI, no Blinc and no store, so it builds and tests in a light slot
(`cargo test -p fs-text-html`). The crate has the target name of ADR 0080 (`fs-text-*`), so the move to `fs-text` does not rename it.

## What it does

* **`HtmlExporter`** implements the existing trait `fs_text::Exporter` unchanged (standard settings; ADR 0084 "block to HTML"). `render(&Document)` returns an
  **`HtmlPage`**: fragment `body`, `title`, `description`, `lang`, `toc`, the links and media used, and a `Report` of what could not be done exactly.
  A `Builder` sets the strategies: `LinkResolver`, `MediaResolver`, `RawHtmlPolicy`, language of the generated words.
* **Page frame** (`frame::render_page`, `Frame`): `<html lang>`, charset, CSP in a `<meta>` (a setting, `Csp::Strict` by default: `default-src 'self'`, no script,
  no plug-in, no `<base>`, no form), viewport, title, description, canonical, `hreflang` links (and `x-default`), one style sheet, skip link,
  `header` with language bar (only with two or more variants), `nav` (short), `main` with bread crumb and exactly one `h1`, `footer`. No script and no external resource.
* **Short navigation** (`nav`): the page shows the top level, the path to itself (an ancestor shows only the next step of the path), its siblings and its children, never the whole tree
  (test: at most 20 KB for a tree of 8420 pages). The full tree belongs on a contents page of the site builder.
* **Theme** (`theme::CSS`): one fixed style sheet, system fonts, light and dark, visible focus; no `@import`, no `url(`.
* **Pass-through** (`pass`): a page of the user (HTML, maybe with its own script) is an original (ADR 0030, 0086) and is copied **byte for byte**; it is never parsed,
  framed or extended (no meta, no CSP). Before that: the path rule (`PassPath`: relative, no `..`, `[A-Za-z0-9._-]`, no hidden part except a top `.well-known`),
  the media table by extension (`MediaType`; unknown extension = error, never `application/octet-stream`), the check of the content against the extension (`Prober`: magic bytes,
  UTF-8, no NUL, `<svg>`), the set of all paths (`PassSet`: duplicates, letter-case twins, file against folder, reserved prefixes), and the file system rules (`copy_file`: no link on the
  way, a regular file with one name only, size limit, output created new and removed again on a refusal). The result carries size, SHA-256 and media type for the manifest.
  HTML, script and SVG are `is_active()`; they are not searched for scripts (decision of the user), the manifest marks them.

## Rules of the export (the answers to "per mark")

| Part | Rule |
|---|---|
| Heading id | ASCII slug (`ä ö ü ß` as `ae oe ue ss`, other non-ASCII dropped, empty = `section`), unique per page (`-2`, `-3`), never `main nav skip content top footer`, never `fn-…`/`fnref-…`; `{#id}` of the heading is used if it is a plain id |
| Link | scheme only `http https mailto` or relative (`javascript:` `data:` `vbscript:` `file:`, `//host`, `\`, control characters refused); refused or unknown (`Missing`/`NotPublic`) = text without link + report entry |
| Image | address from the `MediaResolver` (default: relative only, so a page loads nothing from outside); no alt text = report entry; `width`/`height` when the resolver knows them |
| Raw HTML (block and mark) | allow-list `details summary sub sup kbd u mark br`, attributes `lang` and `title` only, everything else written as **text** and reported; comments dropped; open tags are closed at the end of a block sequence or mark; a closing tag closes only what its own sequence opened; `RawHtmlPolicy::EscapeAll` writes all as text. A `<details>` may stay open over several blocks of one sequence |
| Link reference definitions | Markdown source (`unparsed` blocks), not written |
| Layer marks (`Passage`, `Explanation`, `Variable`) | not exported (ADR 0042): the original text stays; an unfilled variable is reported |
| Footnotes | numbered by first reference, list at the end with back links; reference without definition = `?` + report; definition nobody uses = left out + report |
| Tasks | `<input type="checkbox" disabled>` in a `<label>` with the text of the first paragraph |
| Table | `thead`/`tbody`, alignment as class (`al-l al-c al-r`, no `style`: CSP) |
| Callout | `<aside role="note">` with title; folded callouts `<details>` without script |

## Several sites

One domain may carry several independent sites (the wiki and the landing page, as sites of their own, on a subdomain or under a path). The crate has no state shared between pages
and no host name: everything that belongs to one site comes in through `Frame` (canonical, alternates, `nav`, style sheet, footer) and through the `LinkResolver`. So each site has its
own input, output folder, navigation (the short navigation cuts inside one site; a link to another site exists only if the site's configuration names it), sitemap and `hreflang`.
Links are relative or built from the `host` and `path_prefix` of the site's `site.toml`, never from a fixed name; subdomain and path variants differ only in these two values.
Pass-through is per site and stays byte for byte. `site.toml` (schema 1 of fs-release stage 0, `/srv/FreeSynergy/.scratch/site-toml-schema.md`) holds `names` (primary, aliases),
`dns.records`, `acme`, `csp`; for several sites per domain the fields `site.id`, `host`, `path_prefix`, `languages`, `default_language` are **proposed and need a schema bump (schema 2)**
because schema 1 refuses unknown keys by design. That agreement is open; no host name is in code or tests (only in fixtures).
The later `DnsProvider` of fs-server (ADR 0088) reads only `names` and `dns.records` of each site's `site.toml`; this crate neither writes nor reads DNS data.

## Pattern map

| Module | Pattern | Why |
|---|---|---|
| `exporter` | Adapter (`Exporter`) + Builder | our blocks in, HTML out; the strategies are handed in |
| `resolve` | Strategy | link and media targets belong to the caller (site builder) |
| `raw`, `rawtag` | Strategy (closed enum) + scanner | what happens to raw HTML; safe by construction (not an allowed tag = text) |
| `render`, `render_parts`, `leaf`, `table` | Template method + Visitor | walk of the container tree; one `match` per container and block kind |
| `tree` | Composite (rebuilt) | the flat block list as a tree of containers |
| `inline`, `atoms` | Visitor over a sweep | marks to correctly nested tags (overlap = close and reopen) |
| `state` | Value object (accumulator) | ids, footnote numbers, used links and media, report |
| `frame` | Template method | the fixed sequence around the body |
| `nav` | Composite + Visitor | one walk writing only the short form |
| `pass` | Facade; `path` Value object, `media` Registry + State, `collect` Specification | the one safe entry `copy_file` |
| `url` | Specification | pure predicate |
| `slug`, `page`, `report`, `escape`, `words`, `theme`, `pass::error` | Value object / none | plain data and pure functions |

## Decisions and limits

* No highlighting of code (stage 1: only escaping). No image conversion.
* Own small scanner for raw HTML instead of `ammonia`/`html5ever`: the allow-list is eight tags, and the scanner never drops text (not allowed = escaped text), so the light slot stays light. If the list grows, revisit.
* The model does not keep an image with empty text and no mark range at all (`![](x.png)` is read as an image mark of length zero); such an image is written with `alt=""` and reported.
* The counter-check against `pulldown-cmark` (dev-dependency) compares 35 CommonMark/GFM samples and the corpus files without extension as a normalised token list; named differences: heading `id`, `class`, `scope`, `loading`, `style`, the label and the form of the task box.
* Open for the next cores: the site builder (`fs-text-site`: `SiteSource`, `FolderSource`, page paths, variants, pretty link resolver, media folder, sitemap, 404, manifest), the look after the reference dossier, `lastmod` (needs a real source), the check of the media table against the type Zentinel sends (test in Podman).

## State

Core 1 (this crate) is built and green: exporter, frame, short navigation, theme, pass-through; tests `blocks`, `golden`, `cmark`, `security` (proptest), `frame`, `pass`, `pass_fs`.
Not done: everything of `fs-text-site`.
