#![allow(clippy::try_err)]
extern crate getopts;
use getopts::Options;
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

#[derive(Clone, Copy)]
struct PackageConf<'a> {
    name: &'a str,
    post_build: fn(conf: &Config) -> Result<(), DynError>,
}

const PACKAGES_CONF: &[PackageConf] = &[PackageConf {
    name: "eg-worker-rs",
    post_build: |conf| {
        let lib_file_name = [&conf.lib_prefix(), "eg_worker_rs", &conf.lib_suffix()].concat();
        let subs: &[(&str, &str)] = &[("@LIB_FILE_NAME@", &lib_file_name)];
        let src_dir = workspace_root().join("eg-worker-rs");
        let out_dir = conf.build_dir().join("lv2").join("eg-worker-rs");
        fs::create_dir_all(&out_dir).unwrap();
        subst(
            src_dir.join("manifest.ttl"),
            out_dir.join("manifest.ttl"),
            subs,
        )
        .unwrap();
        for e in &["worker.ttl"] {
            fs::copy(src_dir.join(e), out_dir.join(e)).unwrap();
        }
        fs::copy(
            conf.build_dir().join(&lib_file_name),
            out_dir.join(&lib_file_name),
        )
        .unwrap();
        Ok(())
    },
}];

struct Config<'a> {
    subcommand: String,
    target: String,     // target-triple
    target_dir: String, // directory for all generated artifact
    release: bool,
    packages_conf: Vec<PackageConf<'a>>,
    opts: Options,
}

impl<'a> Config<'a> {
    //build config from environment variable and passed argument
    fn from_env() -> Result<Self, DynError> {
        let mut args = env::args();
        let subcommand = if let Some(arg) = args.nth(1) {
            arg
        } else {
            String::from("")
        };

        let mut opts_args = Vec::<String>::new();
        for e in args {
            if e == "--" {
                break;
            }
            opts_args.push(e);
        }

        let mut opts = Options::new();
        opts.optmulti("p", "project", "project to build", "NAME");
        opts.optflag("", "all", "build all projects");
        opts.optflag("", "release", "build in release mode, with optimization");
        opts.optopt("", "target", "build for the target triple", "TRIPLE");
        opts.optopt(
            "",
            "target-dir",
            "directory for all generated artifacts",
            "DIRECTORY",
        );
        opts.optflag("h", "help", "print this help menu");
        let matches = opts.parse(&opts_args)?;

        let target = if let Some(s) = matches.opt_str("target") {
            s
        } else if let Some(var) = env::var_os("CARGO_BUILD_TARGET") {
            var.into_string().unwrap()
        } else {
            String::from("")
        };

        let target_dir = if let Some(s) = matches.opt_str("target-dir") {
            s
        } else if let Some(var) = env::var_os("CARGO_TARGET_DIR") {
            var.into_string().unwrap()
        } else if let Some(var) = env::var_os("CARGO_BUILD_TARGET_DIR") {
            var.into_string().unwrap()
        } else {
            String::from("target")
        };

        let release = matches.opt_present("release");

        let packages_conf = if matches.opt_present("all") || !matches.opt_present("project") {
            PACKAGES_CONF.iter().copied().collect::<Vec<PackageConf>>()
        } else {
            let mut tmp = Vec::<PackageConf>::new();
            let project = matches.opt_strs("p");
            'proj_loop: for proj in project {
                for pkg_conf in PACKAGES_CONF {
                    if proj == pkg_conf.name {
                        tmp.push(*pkg_conf);
                        continue 'proj_loop;
                    }
                }
                return Err(format!("No project named `{}", proj).into());
            }
            tmp
        };

        Ok(Self {
            subcommand,
            target,
            target_dir,
            release,
            packages_conf,
            opts,
        })
    }

    fn print_help(&self) {
        let brief = "Usage: cargo xtask SUBCOMMAND [options]";
        let mut usage = self.opts.usage(&brief);
        let subcommand = "
     Subcomands are:
         build   build lv2 project(s)

";
        usage.push_str(&subcommand);
        print!("{}", usage);
    }

    fn build_dir(&self) -> PathBuf {
        let profile_dir = if self.release { "release" } else { "debug" };
        workspace_root()
            .join(&self.target_dir)
            .join(&self.target)
            .join(profile_dir)
    }

    fn packages_conf(&self) -> Vec<PackageConf> {
        self.packages_conf.clone()
    }

    fn lib_prefix(&self) -> String {
        let prefix = if self.target.contains("apple") {
            "lib"
        } else if self.target.contains("windows") {
            ""
        } else if cfg!(target_vendor = "apple") {
            "lib"
        } else if cfg!(target_os = "windows") {
            ""
        } else {
            "lib"
        };
        String::from(prefix)
    }

    fn lib_suffix(&self) -> String {
        let suffix = if self.target.contains("apple") {
            ".dylib"
        } else if self.target.contains("windows") {
            ".dll"
        } else if cfg!(target_vendor = "apple") {
            ".dylib"
        } else if cfg!(target_os = "windows") {
            ".dll"
        } else {
            ".so"
        };
        String::from(suffix)
    }
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let mut conf = Config::from_env()?;
    match conf.subcommand.as_ref() {
        "build" => build(&mut conf)?,
        "debug" => debug(&mut conf)?,
        _ => conf.print_help(),
    }
    Ok(())
}

fn build(conf: &mut Config) -> Result<(), DynError> {
    let mut cargo_args = Vec::<String>::new();
    if conf.release {
        cargo_args.push(String::from("--release"));
    }
    if conf.target != "" {
        cargo_args.push(String::from("--target"));
        cargo_args.push(conf.target.clone());
    }
    cargo_args.push(String::from("--target-dir"));
    cargo_args.push(conf.target_dir.clone());

    for p in conf.packages_conf() {
        cargo_args.push(String::from("-p"));
        cargo_args.push(String::from(p.name));
    }
    println!("Building binarie(s)");
    cargo("build", &cargo_args)?;
    println!("Post build step(s)");
    for p in conf.packages_conf() {
        (p.post_build)(conf)?;
    }
    println!("Finished");
    println!();
    Ok(())
}

//substitute tokens in a file
fn subst<P: AsRef<Path>, Q: AsRef<Path>>(
    in_path: P,
    out_path: Q,
    subs: &[(&str, &str)],
) -> Result<(), DynError> {
    let mut in_file = BufReader::new(File::open(in_path)?);
    let mut out_file = BufWriter::new(File::create(out_path)?);
    let mut buf = String::new();
    while in_file.read_line(&mut buf).unwrap() != 0 {
        for (token, value) in subs {
            buf = buf.replace(token, value);
        }
        write!(out_file, "{}", buf)?;
        buf.clear();
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

fn debug(_conf: &mut Config) -> Result<(), DynError> {
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
        Err(format!("cargo {} failed", cmd))?;
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
