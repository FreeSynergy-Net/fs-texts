# fs-text-site - Design

`fs-text-site` builds a static website from a folder: pages (Markdown) in several languages, media of the pages, and files of the user that are published **byte for byte**. It is core 2 of the website export (stage 1, plan `plan-website-stufe1-2.md` part A); core 1 is [`fs-text-html`](../fs-text-html) (text to HTML, page frame, pass-through checks). The crate has the target name of ADR 0080 (`fs-text-*`). It has no GUI, no Blinc, no store, no network, no DNS code, no host name in code (host names appear only in fixtures).

`fs-text-site build <source> <output>` builds one site; `fs-text-site build-all <sites-root> <output-root>` checks **all** `site.toml` files together and builds each into `<output-root>/<site.id>`. The modification time of every output file comes from `SITE_MTIME` (or `--mtime`), never from the clock; without it the program refuses (exit 2).

## Source layout (adapter `FolderSource`)
```
<root>/site.toml                  schema 1 or 2 (see /srv/FreeSynergy/.scratch/site-toml-schema-v2.md)
<root>/content/<lang>/**.md       pages: a/b.md and a/b/index.md are the page a/b (both = error)
<root>/content/<lang>/**/.pages   optional order of siblings (`nav:` list, `- name` or `- Title: name`)
<root>/content/<lang>/**.<ext>    media of the pages (png jpg gif webp avif pdf txt only; nothing that runs)
<root>/files/**                   files of the user, published at the same path, byte for byte
```
Page addresses must be lower case `[a-z0-9._-]`: nothing is renamed silently, a wrong name is listed and stops the build.

## Output (per site, one folder)
`/index.html` (language choice, or the user's own `files/index.html`), `/<lang>/<page>/index.html`, `/<lang>/inhalt/` (de) or `/<lang>/contents/` (other: the full tree; pages show only the short navigation), `/_fs/theme.<sha256-16>.css`, `/_fs/media/<sha256-16>.<ext>`, the passed files, `/404.html`, `/sitemap.xml` (`xhtml:link` alternates, no `lastmod`), `/robots.txt` (only for a site at the root of its host; the file is only read there), `/site.toml` (written back as schema 1 for the fs-release check), `/site-manifest.json` (path, sha256, bytes, media type, kind `page|media|passthrough|generated`, `active` for passthrough, source revision; sorted, no time).

## Several sites on one domain
Each site has its own input, output folder, navigation, sitemap, `hreflang` and `x-default`. A site may lie at a subdomain or under a path of the same host (landing at `/`, wiki at `/wiki`); `SiteSet` rejects the same `(name, path)`, a duplicate `site.id`, and an inner site whose first segment is a name the outer site generates itself. The outer site reserves the inner prefix against its own pass-through files. Links between sites are not resolved (a link to another site is a dead link until the configuration names it).

## Safety rules (each with a test and a mutation probe)
* A passed file: path rule, media table by extension (unknown = error), content against extension, size limit, **no symlink anywhere on the way, no hard link, regular files only** (core 1: opened with `O_NOFOLLOW|O_NONBLOCK`, checked on the descriptor, folder swap detected by comparing canonical paths), nothing overwritten, the hash of the second copy must equal the hash measured first.
* Reserved paths are built into `fs-text-html::pass::ALWAYS_RESERVED` (`_fs`, `site.toml`, `site-manifest.json`, `sitemap.xml`, `.well-known/acme-challenge`), plus the language folders, the nested sites, and `index.html`/`404.html`/`robots.txt` unless the user brings them. Under `.well-known` only `security.txt` passes.
* Active files (HTML, JS, SVG) are not searched for script (decision of the user) but need `csp.mode = "passthrough"` for their folder (`[[csp.area]]`), else the build stops. Page media may not run anything.
* A declared checksum of the source (stage 2 `FileRef`) that does not match stops the build.
* The output folder must be new or empty; after a refusal nothing is left behind.

## Determinism
Sorted everywhere (`BTreeMap`, sorted `read_dir`), no clock, no hash-map order. Test: one process builds twice and compares bytes, modes and times; **two separate processes** (the binary) build and compare; another `SITE_MTIME` changes only the times.

## Media types against Zentinel
Zentinel 26.11_2 sends the type of `mime_guess` 2.0.5 (`from_path(..).first_or_octet_stream()`, no charset). `tests/zentinel_mime.rs` compares our table with that crate at the same pinned version. Result: two entries differed (`.mjs`, `.xml`); the table of core 1 now says what Zentinel sends (`application/javascript`, `text/xml`); the generated `site.toml` is `text/x-toml`. A user page without a `<meta charset>` is sent without a charset by the server; that is the user's file and is not changed.

## Honest limits
* Response headers (`frame-ancestors`, `X-Content-Type-Options`, ...) come from the server, not from the pages. Zentinel 26.11_2 does not send them on static routes (measured), so the CSP exists only as a `<meta>` in generated pages; there is no clickjacking protection and no nosniff yet.
* The short navigation lists at most 40 siblings per level (code, not only test); the contents page lists everything.
* No "translation is older" notice (needs `source_rev`, stage 2); only "variant missing" through hreflang and the language bar.
* No image size, no conversion, no highlighting. Navigation is cloned per language, not per page (one `Frame` per language).
* The race "file changes between the check and the copy" is detected by comparing hashes, but a mutation test cannot provoke it.
* Not measured on the whole fs-documentation (8420 pages); only on fixtures and unit trees.

## Pattern map
| Module | Pattern |
|---|---|
| `source`, `folder`, `walk` | Port `SiteSource` and Adapter `FolderSource` (stage 2 adds adapters for the store and fs-files; the builder does not change) |
| `config`, `raw`, `lang`, `host` | Builder from a raw typed form into Value objects (`SiteConfig`, `Host`, `PathPrefix`, `Lang`) |
| `sites` | Specification over a set of sites |
| `index` | Strategy implementations (`PageLinks`, `PageMedia`) over one shared read-only index |
| `tree` | Builder of a Composite (navigation) |
| `prepare` / `write`, `write_parts` | Two-phase Builder (check everything in memory, then write); `Writer` = Builder steps over one context |
| `outfs` | Unit of work with a ledger (feeds the manifest), fixed modes and times |
| `gen`, `words` | Template method per generated file |
| `report` | Value object with gates (`Policy`: dead links, missing alt, missing media) |
