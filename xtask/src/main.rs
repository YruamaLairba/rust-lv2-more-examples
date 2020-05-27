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
    dir: &'a str,                            //relative from the workspace root
    prefix_tag: &'a str,                     //tag used for prefix replacement
    suffix_tag: &'a str,                     //tag used for suffix replacement
    template_files: &'a [&'a str],           //path relative to package dir
    template_subs: &'a [(&'a str, &'a str)], //additionnal tag/value
    resources: &'a [&'a str],                //additionnal file to copy into lv2 folder
}

const PACKAGES_CONF: &[PackageConf] = &[PackageConf {
    name: "eg-worker-rs",
    dir: "eg-worker-rs",
    prefix_tag: "@LIB_PREFIX@",
    suffix_tag: "@LIB_SUFFIX@",
    template_files: &["manifest.ttl"],
    template_subs: &[],
    resources: &["worker.ttl"],
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

        let mut packages_conf = Vec::<PackageConf>::new();
        let project = matches.opt_strs("p");
        for proj in project {
            for pkg_conf in PACKAGES_CONF {
                if proj == pkg_conf.name {
                    packages_conf.push(*pkg_conf);
                }
            }
        }

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
    println!("Building template(s)");
    build_templates(conf)?;
    println!("Copying ressource(s)");
    build_resources(conf)?;

    println!();

    Ok(())
}

fn build_templates(conf: &mut Config) -> Result<(), DynError> {
    for p in conf.packages_conf() {
        let out_dir = conf.build_dir().join("lv2").join(&p.dir);
        fs::create_dir_all(&out_dir).unwrap();
        for tf in p.template_files {
            let in_path = workspace_root().join(&p.dir).join(&tf);
            let out_path = out_dir.join(Path::new(&tf));
            let mut template = BufReader::new(File::open(in_path).unwrap());
            let mut output = BufWriter::new(File::create(out_path).unwrap());
            let mut buf = String::new();
            while template.read_line(&mut buf).unwrap() != 0 {
                for (token, value) in p.template_subs {
                    buf = buf.replace(token, value);
                }
                buf = buf.replace(p.prefix_tag, &conf.lib_prefix());
                buf = buf.replace(p.suffix_tag, &conf.lib_suffix());
                write!(output, "{}", buf).unwrap();
                buf.clear();
            }
        }
    }
    Ok(())
}

fn build_resources(conf: &mut Config) -> Result<(), DynError> {
    for p in conf.packages_conf() {
        let out_dir = conf.build_dir().join("lv2").join(&p.dir);
        fs::create_dir_all(&out_dir).unwrap();
        for rs in p.resources {
            let in_path = workspace_root().join(&p.dir).join(&rs);
            let out_path = out_dir.join(Path::new(&rs));
            fs::copy(in_path, out_path)?;
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
