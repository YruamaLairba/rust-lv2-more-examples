#![allow(clippy::try_err)]
use std::env;
use std::ffi::OsStr;
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::process::Command;

type DynError = Box<dyn std::error::Error>;

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    let mut args = env::args_os();
    args.nth(1);
    if let Some(task) = task {
        match task.as_str() {
            "build" => build(args)?,
            "debug" => debug()?,
            _ => print_help(),
        }
    } else {
        print_help();
    }
    Ok(())
}

fn print_help() {
    eprintln!(
        "Tasks:

build            build lv2 bundle(s)
"
    )
}

fn build<I, S>(args: I) -> Result<(), DynError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut args = args.into_iter();
    for e in args.by_ref() {
        print!("{:?} ", e.as_ref().to_string_lossy());
    }
    println!();
    cargo("build", args)?;

    Ok(())
}

macro_rules! print_env {
    ( $x:expr) => {{
        println!(
            stringify!($x {}),
            env::var(stringify!($x)).unwrap_or_else(|e| format!("{}", e))
        );
    }};
}

fn debug() -> Result<(), DynError> {
    print_env!(CARGO);
    print_env!(CARGO_MANIFEST_DIR);
    print_env!(CARGO_PKG_VERSION);
    print_env!(CARGO_PKG_VERSION_MAJOR);
    print_env!(CARGO_PKG_VERSION_MINOR);
    print_env!(CARGO_PKG_VERSION_PATCH);
    print_env!(CARGO_PKG_VERSION_PRE);
    print_env!(CARGO_PKG_AUTHORS);
    print_env!(CARGO_PKG_NAME);
    print_env!(CARGO_PKG_DESCRIPTION);
    print_env!(CARGO_PKG_HOMEPAGE);
    print_env!(CARGO_PKG_REPOSITORY);
    print_env!(OUT_DIR);
    print_env!(TARGET);
    print_env!(CARGO_CFG_TARGET_OS);
    Ok(())
}

fn cargo<C, I, S>(cmd: C, args: I) -> Result<(), DynError>
where
    C: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(project_root())
        .arg(cmd)
        .args(args)
        .status()?;

    if !status.success() {
        Err("cargo build failed")?;
    }
    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
