# UX Prototype

Sketches for the interface defined in [ui-design.md](ui-design.md). All
interface labels are English. `*` marks the current location. Actions are
ordinary links; nothing opens as an overlay.

## Repository list — `/`

```text
projects

Name              Description                         Updated (UTC)
gitview           Repository browsing                 2026-10-13 12:00
cgit              Hyperfast Git frontend              2026-10-10 09:20
```

## Repository home — `/gitview`

Redirects to the default branch root below, replacing the old overview.

## Root directory — `/gitview/-/tree/main`

```text
projects / gitview · tree                          main · refs
A small, read-only Git browser.
History · Tags · Branches

Mode    Name                       Size
040000  src/                         --
100644  README.md                 1.0 KiB

------------------------------------------------------------
README.md
# gitview
A small, read-only Git browser.
```

The description row is omitted when no description is configured.
The top repository link opens the default branch root. In a subdirectory,
the repository breadcrumb opens the tree root at the currently viewed revision.

## Directory — `/gitview/-/tree/main/src`

```text
projects / gitview · tree                          main · refs
gitview / src
History

Mode    Name                       Size
040000  ../                          --
040000  page/                        --
100644  main.rs                    3.1 KiB
100755  check.sh  Executable       1.0 KiB
```

## File — `/gitview/-/blob/main/src/main.rs`

```text
projects / gitview · blob                          main · refs
gitview / src / main.rs
History · Raw · Diff · Blob

172 lines · 5.8 KiB

  1  use axum::Router;
  2
  3  fn main() {
```

## Markdown — `/gitview/-/blob/main/README.md`

```text
projects / gitview · blob                          main · refs
gitview / README.md
History · Raw · Blob · Preview

12 lines · 1.0 KiB

# gitview
A small, read-only Git browser.
```

`Blob` opens `?view=code` with line numbers. The current action is black text,
not a link; all entries share one action row. Directory README previews have
no view selector; their filename links to this file page.

## Log — `/gitview/-/log/main/src/main.rs`

```text
projects / gitview · log                           main · refs
gitview / src / main.rs
Tree · History · Raw

Commit    Subject                        Author  Time (UTC)
4badb69a  Fix configuration loading      qaq     2026-10-13 12:00
3c2a1f0b  Extract page handlers          qaq     2026-10-12 09:20

Older commits
```

## Commit — `/gitview/-/commit/9e8d7c6d`

```text
projects / gitview · commit                        9e8d7c6d
Browse tree · Parent · Raw patch

Fix configuration loading
9e8d7c6d · qaq <qaq@example.com> · 2026-07-14 09:32:11 +08:00

The loader now reports invalid configuration before startup.

Commit      9e8d7c6d1a2b3c4e5f60718293a4b5c6d7e8f901
Parents     3c2a1f0b

Changes
2 files changed · +13 −3

File           Status    Add  Del  Content
src/config.rs  modified  +13  −2   View file
old.conf       deleted   +0   −1   View before deletion
```

## Version selection — `/gitview/-/refs/blob/src/main.rs?to=feature%3Alogin`

```text
projects / gitview

Switch version
Continue viewing content of src/main.rs.

Branches
Name             Commit    Time (UTC)
main  default    4badb69a  2026-10-13 12:00
feature/login    3c2a1f0b  2026-10-12 09:20

Tags
Name    Commit    Time (UTC)
v1.0    5f6e7d8f  2026-10-01 08:00
```

Choosing a row keeps the mode and path. It never returns to the root or
switches to another view.

## Narrow screens

The status row wraps if needed. The breadcrumb hides its middle:

```text
projects / gitview · blob
main · refs
… / src / main.rs
History · Raw · Diff · Blob
```

Source, diffs, and tables scroll inside their own regions. The page itself
never scrolls horizontally.
