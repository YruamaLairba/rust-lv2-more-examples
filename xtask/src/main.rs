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

#[derive(Clone)]
struct PackageConf {
    name: String,
    dir: String,                 //relative from the workspace root
    template_files: Vec<String>, //path relative to package dir
    template_subs: Vec<(String, String)>,
}

impl PackageConf {
    fn with_name(name: &str) -> Self {
        Self {
            name: String::from(name),
            dir: String::from(name),
            template_files: vec![],
            template_subs: vec![],
        }
    }

    fn add_template_file(&mut self, file_name: &str) {
        self.template_files.push(String::from(file_name));
    }

    fn add_template_sub(&mut self, token: &str, value: &str) {
        self.template_subs
            .push((String::from(token), String::from(value)));
    }
}

struct Config {
    subcommand: Option<String>,
    target: Option<String>,     // target-triple
    target_dir: Option<String>, // directory for all generated artifact
    release: bool,
    packages: Vec<String>,
    //i didn't find better place to store it
    packages_conf: Vec<PackageConf>,
}

impl Config {
    fn _new() -> Self {
        Self {
            subcommand: None,
            target: None,
            target_dir: None,
            release: false,
            packages: vec![],
            packages_conf: vec![],
        }
    }

    //build config from environment variables
    fn _vars(mut self) -> Result<Self, DynError> {
        //let vars = env::vars();
        //for (key, value) in vars {
        //    match key {}
        //}
        Ok(self)
    }

    fn _arg_package(&mut self, p: Option<String>) -> Result<(), DynError> {
        if let Some(p) = p {
            self.packages.push(p);
        } else {
            return Err("The argument '--package' requires a value".into());
        }
        Ok(())
    }

    fn _arg_target(&mut self, tg: Option<String>) -> Result<(), DynError> {
        if self.target.is_some() {
            return Err("The argument '--target' was provided more than once".into());
        }
        if tg.is_some() {
            self.target = tg;
        } else {
            return Err("The argument '--target' require a value".into());
        }
        Ok(())
    }

    fn _arg_target_dir(&mut self, tg: Option<String>) -> Result<(), DynError> {
        if self.target_dir.is_some() {
            return Err("The argument '--target-dir' was provided more than once".into());
        }
        if tg.is_some() {
            self.target_dir = tg;
        } else {
            return Err("The argument '--target-dir' require a value".into());
        }
        Ok(())
    }

    //build config from argument passed to app
    fn _args(mut self) -> Result<Self, DynError> {
        let mut args = env::args();
        self.subcommand = args.nth(1);
        while let Some(arg) = args.next() {
            match arg.as_ref() {
                "-p" | "--package" => self._arg_package(args.next())?,
                "--release" => self.release = true,
                "--target" => self._arg_target(args.next())?,
                "--target-dir" => self._arg_target_dir(args.next())?,
                _ => return Err(format!("Unexptected argument: '{}'", arg).into()),
            }
        }
        Ok(self)
    }

    fn _packages_conf(mut self) -> Result<Self, DynError> {
        let mut packages_conf = vec![];
        let mut eg_worker_rs = PackageConf::with_name("eg-worker-rs");
        eg_worker_rs.add_template_file("manifest.ttl.in");
        eg_worker_rs.add_template_sub("@LIB_FILE_NAME@", &self.lib_filename("eg-worker-rs")?);
        packages_conf.push(eg_worker_rs);

        if self.packages.is_empty() {
            self.packages_conf = packages_conf;
        } else {
            for p_name in &self.packages {
                for p in &packages_conf {
                    if &p.name == p_name {
                        self.packages_conf.push(p.clone());
                    }
                }
            }
        }
        Ok(self)
    }

    //build config from environment variable and passed argument
    fn from_env() -> Result<Self, DynError> {
        Ok(Config::_new()._vars()?._args()?._packages_conf()?)
    }

    fn build_dir(&self) -> PathBuf {
        let target_dir = self.target_dir.as_deref().unwrap_or_default();
        let target = self.target.as_deref().unwrap_or("target");
        let profile_dir = if self.release { "release" } else { "debug" };
        workspace_root()
            .join(target_dir)
            .join(target)
            .join(profile_dir)
    }

    fn packages_conf(&self) -> Vec<PackageConf> {
        self.packages_conf.clone()
    }

    fn lib_filename(&self, project_name: &str) -> Result<String, DynError> {
        let (prefix, suffix) = if let Some(tg) = self.target.as_deref() {
            if tg.contains("apple") {
                ("lib", ".dylib")
            } else if tg.contains("windows") {
                ("", ".dll")
            } else {
                ("lib", ".so")
            }
        //if no target provided, use xtask target information wich are host information
        } else if cfg!(target_vendor = "apple") {
            ("lib", ".dylib")
        } else if cfg!(target_os = "windows") {
            ("", ".dll")
        } else {
            ("lib", ".so")
        };

        let base_name = project_name.replace("-", "_");
        Ok([prefix, &base_name, suffix].concat())
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
    if let Some(task) = &conf.subcommand {
        match task.as_ref() {
            "build" => build(&mut conf)?,
            "debug" => debug(&mut conf)?,
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

fn build(conf: &mut Config) -> Result<(), DynError> {
    let mut cargo_args = Vec::<String>::new();
    if let Some(target) = conf.target.as_deref() {
        cargo_args.push(String::from("--target"));
        cargo_args.push(String::from(target));
    }
    if let Some(target_dir) = conf.target_dir.as_deref() {
        cargo_args.push(String::from("--target-dir"));
        cargo_args.push(String::from(target_dir));
    }
    for p in conf.packages_conf() {
        cargo_args.push(String::from("-p"));
        cargo_args.push(p.name);
    }

    cargo("build", &cargo_args)?;
    build_templates(conf)?;

    println!();

    Ok(())
}

fn build_templates(conf: &mut Config) -> Result<(), DynError> {
    for p in conf.packages_conf() {
        let out_dir = conf.build_dir().join("lv2").join(&p.dir);
        for tf in p.template_files {
            let template_in_path = workspace_root().join(&p.dir).join(&tf);
            let template_out_path = out_dir.join(Path::new(&tf).file_stem().unwrap());
            subs(&template_in_path, &template_out_path, &p.template_subs)?;
        }
    }
    Ok(())
}

fn subs(template: &Path, output: &Path, subs: &[(String, String)]) -> Result<(), DynError> {
    fs::create_dir_all(output.parent().unwrap()).unwrap();
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
