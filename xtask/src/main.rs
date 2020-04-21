#![allow(clippy::try_err)]
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
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
    let args = env::args().collect::<Vec<_>>();
    let task = args.get(1);
    let args = args.get(2..).unwrap_or_default();
    if let Some(task) = task {
        match task.as_ref() {
            "build" => build(&args)?,
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

fn build(args: &[String]) -> Result<(), DynError> {
    //cargo("build", args)?;
    let mut args_iter = args.iter();
    while let Some(e) = args_iter.next() {
        if e == "-p" || e == "--package" {
            if let Some(p) = args_iter.next() {
                print!("{:?},", p);
                template_build(workspace_root().join(p))?;
            }
        }
    }
    println!();

    Ok(())
}

fn template_build<P: AsRef<Path>>(project_path: P) -> Result<(), DynError> {
    let entries = fs::read_dir(&project_path)?;
    for entry in entries {
        let path = entry?.path();
        if path.is_file() && path.extension() == Some("in".as_ref()) {
        println!("{:?}",path);
            let out_path = workspace_root()
                .join("target/lv2/")
                .join(project_path.as_ref().file_stem().unwrap())
                .join(path.file_stem().unwrap());
        println!("{:?}",out_path);
            subst_file(path, &out_path, &[("@LIB_NAME@", "libeg_worker_rs.so")])?;
        }
    }
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

fn cargo(cmd: &str, args: &[String]) -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let status = Command::new(cargo)
        .current_dir(workspace_root())
        .arg(cmd)
        .args(args)
        .status()?;

    if !status.success() {
        Err("cargo build failed")?;
    }
    Ok(())
}

fn workspace_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}

fn subst_file<T, O>(template: T, output: O, subs: &[(&str, &str)]) -> Result<(), DynError>
where
    T: AsRef<Path>,
    O: AsRef<Path>,
{
    fs::create_dir_all(output.as_ref().parent().unwrap()).unwrap();
    let mut template = BufReader::new(File::open(template).unwrap());
    let mut output = BufWriter::new(File::create(output).unwrap());
    let mut buf = String::new();
    while template.read_line(&mut buf).unwrap() != 0 {
        for (token, value) in subs {
            buf = buf.replace(token, value);
        }
        write!(output, "{}", buf).unwrap();
        buf.clear();
    }
    Ok(())
}
