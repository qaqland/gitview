use std::{
    io::{self, Write},
    path::PathBuf,
};

use getopts::Options;
use view_query::{QueryError, get_repository, head_rev, log, open_read_only, path_history};

const USAGE: &str = "usage: view-query -t DB [-n N] NAME [-- PATH]";

struct Args {
    database: Option<PathBuf>,
    limit: usize,
    repo: String,
    path: Option<String>,
}

fn parse_args(
    raw: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<Args, Box<dyn std::error::Error>> {
    let mut opts = Options::new();
    opts.optopt("t", "", "SQLite database (or VIEW_DATABASE)", "DB");
    opts.optopt("n", "", "Maximum commits to print", "N");
    let matches = opts.parse(raw).map_err(|err| format!("{err}\n{USAGE}"))?;
    let limit = match matches.opt_str("n") {
        Some(n) => n
            .parse()
            .map_err(|err| format!("invalid limit '{n}': {err}"))?,
        None => 1,
    };
    let database = matches.opt_str("t").map(PathBuf::from);
    let mut free = matches.free.into_iter();
    let repo = free
        .next()
        .ok_or_else(|| format!("repository name required\n{USAGE}"))?;
    let path = free.next();
    if free.next().is_some() {
        return Err(format!("unexpected argument\n{USAGE}").into());
    }
    Ok(Args {
        database,
        limit,
        repo,
        path,
    })
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let database = args
        .database
        .or_else(|| std::env::var_os("VIEW_DATABASE").map(PathBuf::from))
        .ok_or("database path required: use -t or VIEW_DATABASE")?;
    let mut conn = open_read_only(database)?;
    // Resolve the ref and its history against one database snapshot.
    let tx = conn.transaction()?;
    let repo = get_repository(&tx, &args.repo)?.ok_or(QueryError::RepoNotFound)?;
    let rev = head_rev(&tx, &repo)?;
    let page = match args.path.as_deref() {
        Some(path) => path_history(&tx, repo.id, path, &rev.commit_hash, None, args.limit)?,
        None => log(&tx, repo.id, &rev.commit_hash, None, args.limit)?,
    };
    tx.commit()?;

    let mut out = io::BufWriter::new(io::stdout().lock());
    for hash in page.hashes {
        writeln!(out, "{hash}")?;
    }
    out.flush()?;
    Ok(())
}

fn main() -> std::process::ExitCode {
    let result = parse_args(std::env::args_os().skip(1)).and_then(run);
    let Err(err) = result else {
        return std::process::ExitCode::SUCCESS;
    };
    // Broken pipe just means the reader (`| head`) went away.
    if err
        .downcast_ref::<io::Error>()
        .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe)
    {
        return std::process::ExitCode::SUCCESS;
    }
    eprintln!("error: {err}");
    std::process::ExitCode::FAILURE
}
