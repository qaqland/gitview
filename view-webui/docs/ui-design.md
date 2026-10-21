# UI Design

This is the authoritative UI specification for `view-webui`. Routes are in
[url-design.md](url-design.md); page sketches are in
[ux-prototype.md](ux-prototype.md).

## Principles

- Plain text first. Monospace everywhere, high information density, no decoration.
- Server-rendered HTML with ordinary links and no JavaScript.
- Every view is a bookmarkable, shareable URL that works with browser back.
- No overlays of any kind: no dropdown, popover, tooltip, or modal. Every state
  change is a page navigation.

## Three Header Rows

Path views (`tree`, `blob`, `log`) have three header rows: site/status,
breadcrumb, and actions. `commit` omits the breadcrumb row.

### Status row

- Left: `projects / {repo} · {type}`, with the object type as plain text:
  `tree`, `blob`, `log`, or `commit`. The type is never a link and is never blue.
- `projects` returns to the repository list. The repository name links to
  `/{repo}/-/tree`, which redirects to the default branch root.
- The site/repository links, object type, and right-hand ref share one row;
  they wrap only when the screen is too narrow.
- Right for `tree` / `blob` / `log`: the current ref, then `refs`:
  `main · refs`. The ref text is plain and not blue. `refs` is the only link
  and points at `/{repo}/-/refs/{mode}/{path}`.
- Right for `commit`: the short commit hash. There is no ref switcher and no
  `refs` link.
- A 1px rule separates this row from the breadcrumb row.

### Breadcrumb row

- Format: `repo / dir / subdir / file` with `/` separators in the monospace font.
- The repository crumb returns to the current revision's `tree` root, never
  the default branch. Other ancestors keep the current view for their path.
  The final segment is plain text and not blue.
- At the root, show the repository description in place of the breadcrumb.
  Use the same Git description as the project list; omit this row when empty.
- `commit` hides this row.
- On narrow screens the middle is truncated: `… / net / e1000.c`. The page
  itself never scrolls horizontally.

### Action row

- Secondary path/object actions only, one size smaller than the breadcrumb,
  colored `--text-muted`.
- `tree`: `History`
- At the root, append `Tags · Branches` on the same action row, linking to
  `/{repo}/-/tags` and `/{repo}/-/branches`. Subdirectories omit these links.
- Text `blob`: `History · Raw · Diff · Blob`, followed by `Preview`
  only when preview is available. Markdown/README omit `Diff`; binary files
  omit `Diff` and `Preview`. Existing diff URLs remain available.
- The current action is black, non-clickable text with `aria-current`; all
  actions, including `Blob` and `Preview`, share one row without subgroups.
- `log`: `Tree · History · Raw` (`Tree` goes to the containing directory for a
  file, or to the directory itself)
- `commit`: `Browse tree · Parent · Raw patch`
- Path actions carry the current ref and path; repository tags/branches lists
  are independent of the path. `Parent` is omitted for an
  initial commit.
- There is no rule between the breadcrumb and action rows; they are grouped
  with about `0.25rem` spacing.

Actions change how the current location is viewed. Breadcrumb ancestors move to
another path. They never duplicate each other.

- `blob` → `History` opens the file's `log` with the same path.
- `log` → `Tree` opens the containing directory's `tree`.
- Clicking a directory ancestor keeps the current view (`log` stays `log`),
  except the repository crumb, which always opens the current revision's tree.

## Typography and Links

- Font stack everywhere:
  `"JetBrains Mono", "IBM Plex Mono", "SFMono-Regular", monospace`.
- Body 13px, metadata 12px, line numbers 11px.
- Links use `var(--link)` and have no visible underline by default. Reserve a
  `1px solid transparent` bottom border; hover changes only its color to
  `currentColor`, without shifting layout. Text color and background do not change.
- The current view, current ref, and final breadcrumb segment are not blue.
  Use black or `--text-muted`, optionally bold or a 1px black bottom rule.
- No colored badges, avatars, icon fonts, emoji, cards, rounded corners,
  shadows, gradients, transitions, animations, or hover movement.
- Diff addition/deletion backgrounds apply only to diff lines, never as a
  general emphasis color.

## Header and Footer

- Header and footer use `var(--bg-subtle)`, separated from content by a 1px
  `var(--rule)` border above/below.
- The header contains the three rows above, with `projects / {repo}` in the
  status row rather than a separate site row.
- The footer is 12px `--text-muted` text with no navigation links.
- The internal header rule belongs to the path row; pages without one show
  only the outer header border.

## Views

### Directory (`tree`)

Columns stay `Mode`, `Name`, `Size`. Mode stays visible; directory names sort
first and end with `/`; directories show `--` size. The parent row remains a
local return link. Submodules and symlinks are labeled, not followed.
Only directories (including `../`) and executable files use blue name links;
other file links use `var(--text)` and remain clickable.

Below the file list, render the current directory's README at the current
revision, separated by a rule, without a card. Match names case-insensitively,
with priority `README.md`, `README.markdown`, then `README`; ties use filename
order. Plain `README` is escaped text, not Markdown. Never search parents or
follow symlinks. No README means no extra section. Binary, non-UTF-8, and files
larger than 256 KiB show an explicit unavailable notice and a file link.
Relative links and images resolve from the README's directory at the current
revision, using the same sanitized renderer as file previews.

### File (`blob`)

Show size and line count, without latest-commit metadata, then a line-numbered source table
that scrolls horizontally inside its own region. Line numbers link to the
immutable blob URL at the resolved commit with `#L<n>`. Binary files show size
and a download link. Source is always escaped.

Markdown (`.md`, `.markdown`, case-insensitive) and plain `README` default to
Preview when they are UTF-8 text up to 256 KiB. The header's unified action row
shows `History · Raw · Blob · Preview`, with the current view black
and non-clickable. There is no separate view selector in the content.
`Blob` uses `?view=code` for line-numbered source; new source permalinks include this
query for previewable files. Legacy `#L<n>` links reveal source via CSS even
without the query, selecting `Blob` in the same action row, so existing line
links continue to work without JavaScript.
Oversized or binary files fall back to source/download with an explicit notice.

### Diff

`Diff` shows the unified
diff for the path at the current revision against its first parent. It keeps
the breadcrumb, `blob` status label, and file actions so the current location
is never lost.

### Log (`log`)

Keep the table order `Commit`, `Subject`, `Author`, `Time (UTC)`. Hash and
subject link to the commit. Pagination uses `?after=<hash>`; later pages also
offer `Latest commits`. The heading (`Commit log` or `Log of <path>`) is
screen-reader-only; the visible path is already in the breadcrumb.
There is no global History tab.

### Commit (`commit`)

The status row shows `commit` and the short hash. Show one subject heading,
author/email, authored date, and the message body. Then the changed-file summary
and per-file diffs. Changed-file names jump to their diff anchors; deleted files
offer the parent version. Renames show old → new. At most 600 diff lines are
rendered; truncation is explicit and links to the complete patch. Every parent
is linked.

## Repository Home, Refs, Repositories, Errors

- `/{repo}` temporarily redirects to the default branch tree root. The directory
  listing and README replace the old summary page; description stays in the
  header, the default branch is visible in the status row, and `History`
  provides the commit log. There is no separate overview or recent-commit block.
- `tags` and `branches` are dedicated listing pages; rows open the selected
  revision's tree root. `refs` is exclusively for switching versions and
  shows both lists, linking each row to the same path and view at the target
  revision. It never resets to the root or changes the view mode silently.
  Switching pages have no path breadcrumb; the context line names the view and
  path. Dedicated listings have no path breadcrumb either. Missing target paths
  produce a contextual error. The default marker is plain muted text, not a badge.
- Repository list: Name / Description / Updated (UTC) table with no decoration.
  No visible heading below the `projects` site link; keep a screen-reader-only
  heading. Row timestamps omit the redundant UTC suffix.
- Errors: status, requested location when known, a plain message, and recovery
  links. No stack traces or internal paths.

## Accessibility and Responsiveness

- Keep the same reading order on narrow screens; path rows wrap or truncate in
  the middle, never by scrolling the page.
- Tables, source, and diffs scroll inside their own containers.
- Use heading levels, table header scopes, `aria-current` on the final
  breadcrumb, labeled navigation, a skip link, and visible focus outlines.
- Never rely on color alone; diff markers `+` and `-` remain literal.
