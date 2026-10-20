# view-index

Index local Git repository metadata into an SQLite database.

## Build

Requires a C23 compiler with `#embed` support (e.g. GCC 15+), Make, Meson 1.3+,
Ninja, and development packages for SQLite 3.37+ and zlib.

```sh
make libgit   # download Git source and compile libgit.a
meson setup build
meson compile -C build
```

## Usage

```sh
export VIEW_DATABASE="$PWD/index.db"    # created automatically; use -t to override

view-index -a /path/to/my-repo  # register a repository
view-index my-repo              # initial or incremental sync
view-index -l                   # list registered repositories
view-index -s my-repo           # show statistics
```
