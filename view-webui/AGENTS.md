# Agent Instructions

## Do not write tests

Do not add or extend tests unless explicitly asked. Verify changes by reading
code and running the app, not by writing new test code.

## Project layout

- `src/web.rs` — axum router; all user-facing routes live here.
- `src/url.rs` — URL encoding helpers and route builders.
- `src/page/` — one module per page, with matching `templates/*.html`.
- `src/data/` — SQLite and git access behind ports in `src/data.rs`.
- `docs/` — URL, UI, and UX design notes; update them when routes change.

## Conventions

- URL shape: `/:repo/-/<op>/<rev>/<path>`. Keep new routes under `/-/`.
- Build links with the helpers in `src/url.rs` and `src/page.rs`; never
  hand-concatenate encoded URLs.
- Keep changes minimal and consistent with the surrounding code.
