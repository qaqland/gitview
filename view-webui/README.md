# view-webui

A read-only web interface for browsing Git repositories indexed by [view-index](../view-index).

## Setup

Create a SQLite database with `view-index` named `view.db`.

## Run

```sh
cargo run -- --database test.db
```

Open http://127.0.0.1:3000 in a browser. All options are CLI flags; see `--help`.

## Styles and templates

`static/view.css` is a plain hand-written stylesheet, embedded into the
binary at build time. Pass `--css static/view.css` to serve the file from
disk instead, re-read on every request — style changes then need only a
browser refresh. Templates are compiled into the binary, so template or
Rust changes need a `cargo run` restart.
