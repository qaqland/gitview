# URL Design

GitView uses flat repository names and a GitLab-style `/-/` operation boundary.
URLs are readable, shareable, server-rendered, and usable without JavaScript.
Navigation behavior is defined in [ui-design.md](ui-design.md).

## Repository and Operation Boundary

```text
/                         repository list
/:repo                    redirect to default branch tree root
/:repo/-/:operation/...   repository operation
```

Repository names do not have `user/repo` namespaces. Percent-encode repository
names and individual path segments, including spaces, Unicode, `#`, `?`, and
literal `%`. Preserve `/` as the path separator. HTML escaping is separate from
URL encoding. Application-generated URLs use the helpers in `src/url.rs`.

## Revision Syntax

```text
main                       branch
feature:login              branch feature/login
tag/v1.0                   tag v1.0
tag/release:2026-06         tag release/2026-06
abcdef12                   unambiguous commit hash prefix (8+ hex characters)
<full commit hash>         immutable commit identity
```

A branch or tag's name occupies one encoded path segment. The `tag/` discriminator
is a separate segment. The UI displays original Git names with slashes; colons
are an internal URL convention, not user-facing labels.

Parsing order remains tag discriminator, hexadecimal commit candidate, branch.
Hash-like branch names still collide with commit candidates; no new revision
syntax is introduced here. The SQLite adapter accepts both slash-containing
indexed names and older colon-form show names, exposing canonical URL names.

## HTML Routes

| Route | Purpose |
| --- | --- |
| `/` | Repository list |
| `/:repo` | Temporary redirect to the default-version tree root |
| `/:repo/-/refs/tree/*path?to=` | Version selection, returning to `tree` |
| `/:repo/-/refs/blob/*path?to=` | Version selection, returning to `blob` |
| `/:repo/-/refs/log/*path?to=` | Version selection, returning to `log` |
| `/:repo/-/branches` | Dedicated branch listing |
| `/:repo/-/tags` | Dedicated tag listing |
| `/:repo/-/refs` | Branches and tags listing; takes no query parameters |
| `/:repo/-/tree` | Temporary redirect to the default-version root |
| `/:repo/-/tree/:rev` | Version root |
| `/:repo/-/tree/:rev/*path` | Directory content |
| `/:repo/-/blob/:rev/*path` | File content; eligible Markdown/README defaults to preview |
| `/:repo/-/blob/:rev/*path?view=code` | Line-numbered source |
| `/:repo/-/log/:rev` | Root commit log |
| `/:repo/-/log/:rev/*path` | Path log |
| `/:repo/-/diff/:rev/*path` | File diff against the first parent |
| `/:repo/-/commit/:hash` | Commit details and per-file diffs |

`/-/history/...` remains available as a legacy alias for `/-/log/...` and is
not linked by the interface.

Blob content links that resolve to a directory redirect temporarily to the
corresponding tree URL. This also supports relative directory links in README
without guessing object kinds from filename extensions. `/blob/:rev` redirects
to the version root.

There is no separate diff route. Changed-file entries use `#diff-<n>` anchors
on the commit page. Text file lines have `#L<n>` anchors. Line-number links use
the full resolved commit hash so branch movement cannot change the reference.
For previewable files they include `?view=code#L<n>`. Legacy `#L<n>` URLs still
reveal source through CSS target handling. `view` accepts only `preview` (the
default) or `code`; unsupported values return 400. Non-previewable files show
source or a binary download regardless of the selected view.

## Log Pagination

```text
/:repo/-/log/:rev/*path?after=<commit-hash>
```

The cursor is excluded from the next page. Older commits uses the next cursor;
Latest commits removes it without changing revision or path. There are no
page-number paths. Content/History switching and ancestor breadcrumbs do not
carry pagination cursors to a different scope.

## Branches & Tags Context

The status row links to a mode- and path-preserving switch page:

```text
/gitview/-/refs/tree
/gitview/-/refs/blob/src%2Fmain.rs
/gitview/-/refs/log/src%2Fmain.rs
```

Parameters:

- path segment `mode`: exactly `tree`, `blob`, or `log`.
- path segment `*path`: source repository-relative path, absent for the root.
- `to`: target revision, added by a selection link.

Selection example:

```text
/gitview/-/refs/blob/src%2Fmain.rs?to=feature%3Alogin
  -> /gitview/-/blob/feature:login/src/main.rs
```

The response validates the target revision and keeps the requested mode and
path. `log` does not require the path to exist in the target tree. Content
modes require the path to be present with the same object kind; otherwise the
response is a contextual 404. It never silently returns to the root or changes
mode.

The switch page does not track the current revision, so `rev` is not a
recognised parameter. Switchers show both branches and tags. Listing pages use
the dedicated
`/-/branches` and `/-/tags` URLs; their rows open the selected revision's tree root.
`/-/refs` lists both branches and tags and ignores query parameters, so the old
`/-/refs?kind=...` and `/-/refs?path=&view=` links show that listing instead of
switching.

Paths must be repository-relative, without empty, dot, parent, or NUL segments.
There is no arbitrary `return_to` URL or external redirect parameter. Query
values are encoded with the standard URL form serializer, independently of
route segments.

## Raw Content and Patch Downloads

```text
/:repo/-/raw/:rev/*path
/:repo/-/raw/:rev/*path?download=1
/:repo/-/commit/:hash.patch
```

- UTF-8 text without NUL bytes: `text/plain; charset=utf-8`.
- Recognized PNG, JPEG, GIF, WebP bytes: corresponding raster image type.
- Other binary content: `application/octet-stream`, attachment disposition.
- `download=1`: force attachment disposition, including for text and images.
- Patch: `text/x-patch; charset=utf-8`, attachment disposition, complete contents.

These responses have no HTML chrome. They use `X-Content-Type-Options: nosniff`
and a sandbox Content Security Policy. HTML/SVG source is not served as active
content. Filenames in attachment headers are UTF-8 percent-encoded.

## README Links

README links resolve relative to the README's directory at the displayed version.
They retain the displayed revision rather than silently switching to a commit
snapshot; line-number permalinks are the explicit immutable navigation path.
Relative links use blob content URLs (directories redirect to tree); image
sources use raw URLs. Root-relative paths refer to the repository root, not to
the site root. External and protocol-relative URLs remain external. Local
heading fragments use prefixed, deduplicated IDs, including links to headings
in other Markdown files. Source-line fragments remain `#L<n>`. Relative content-link query
strings are not forwarded as application control parameters.

## Debug Timing

Every response reports its server-side stage timings through the standard
`Server-Timing` response header, so no query parameter is involved and nothing is
added to the body. Raw content, patch downloads, and CSS carry the same header.

Blob requests report `render.preview` (when rendering a preview) and
`render.source` (source highlighting/escaping), including blocking-pool queue
and scheduling time. Lazy initialization reports `highlight.syntax_init`,
`highlight.markdown_init`, `highlight.theme_init`, and `highlight.css_init` only
on the request performing that initialization. Concurrent requests waiting for
initialization include the wait in their enclosing stage, not as a second init.
Initialization stages are nested inside rendering/Git/CSS stages; do not sum
all metrics as if they were disjoint. `total` is elapsed request time, and each
reported value is rounded independently to whole milliseconds.
