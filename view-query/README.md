# view-query

Synchronous Rust queries over a view-index SQLite database.

## lib

TODO

## bin

The database is built by `view-index`; pick a `NAME` from `view-index -l`.
Pass the database with `-t` or `VIEW_DATABASE`.

```
usage: view-query -t DB [-n N] NAME [-- PATH]
```

Prints one full commit hash per line, newest first. `-n` defaults to 1.
Its output is equivalent to `git log --first-parent -n N --format=%H -- PATH`.

```sh
view-query -t view.db linux                     # latest commit on the default branch
view-query -t view.db -n 100 linux -- Makefile  # last 100 commits touching Makefile
view-query -t view.db linux -- drivers/         # directories end with '/'
```

PATH must exist in the index and is matched exactly (`drivers` != `drivers/`);
an unknown path prints nothing and exits 0.
