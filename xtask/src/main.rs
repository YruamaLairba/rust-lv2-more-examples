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

const PACKAGE_LIST: &[&str] = &["eg-worker-rs"];

fn packages_list() -> Vec<String> {
    let mut vec = Vec::with_capacity(PACKAGE_LIST.len());
    for p in PACKAGE_LIST {
        vec.push(String::from(*p));
    }
    vec
}

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

//const eg_worker_rs: Package = Package {
//    name: "eg-worker-rs",
//    dir: "eg-worker-rs",
//    template_files: &["worker.ttl.in","manifest.ttl.in"],
//    template_subs:&[("@LIB_NAME@","eg-worker-rs")],
//};

fn package_list() -> Vec<PackageConf> {
    let wr = workspace_root();
    let (prefix, ext) = if cfg!(windows) {
        ("", ".dll")
    } else if cfg!(macos) {
        ("lib", ".dylib")
    } else if cfg!(unix) {
        ("lib", ".so")
    } else {
        panic!("Couldn't determine shared library prefix and suffix for that target");
    };
    let mut eg_worker_rs = PackageConf::with_name("eg-worker-rs");
    eg_worker_rs
        .template_files
        .push(String::from("worker.ttl.in"));
    eg_worker_rs
        .template_files
        .push(String::from("manifest.ttl.in"));
    eg_worker_rs.template_subs.push((
        String::from("@LIB_FILE_NAME@"),
        format!("{}{}{}", prefix, "eg_worker_rs", ext),
    ));
    vec![eg_worker_rs]
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

    fn package_list(&self) -> Vec<String> {
        if self.packages.is_empty() {
            packages_list()
        } else {
            self.packages.clone()
        }
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
        dbg!([prefix, &base_name, suffix].concat());
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
    for p in conf.package_list() {
        cargo_args.push(String::from("-p"));
        cargo_args.push(p);
    }
    println!("{:?}", cargo_args);

    cargo("build", &cargo_args)?;
    build_templates(conf)?;

    println!();

    Ok(())
}

fn build_templates(conf: &mut Config) -> Result<(), DynError> {
    for p in conf.package_list() {
        let project_dir: &Path = &p.as_ref();
        let out_dir = conf.build_dir().join("lv2").join(&project_dir);
        let manifest_in_path = workspace_root().join(project_dir).join("manifest.ttl.in");
        dbg!(&out_dir);
        let manifest_out_path = out_dir.join("manifest.ttl");
        let sub = (String::from("@LIB_NAME@"), conf.lib_filename(&p)?);
        subs(&manifest_in_path, &manifest_out_path, &[sub])?;
    }
    Ok(())
}

fn build_template(project: &PackageConf, build_dir: &Path) {
    let project_dir = workspace_root().join(&project.dir);
    let out_dir = build_dir.join("lv2").join(&project.dir);
    for file in &project.template_files {
        let file_path = project_dir.join(&file);
        let file_stem = AsRef::<Path>::as_ref(&file).file_stem().unwrap();
        let out_path = out_dir.join(file_stem);
        subs_file(file_path, out_path, &project.template_subs).unwrap();
    }
}

fn subs(template: &Path, output: &Path, subs: &[(String, String)]) -> Result<(), DynError> {
    dbg!(template);
    dbg!(output);
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

fn template_build<P, B>(project_path: P, build_path: B) -> Result<(), DynError>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    let entries = fs::read_dir(&project_path)?;
    for entry in entries {
        let path = entry?.path();
        if path.is_file() && path.extension() == Some("in".as_ref()) {
            println!("{:?}", path);
            let out_path = build_path
                .as_ref()
                .join("target/lv2/")
                .join(project_path.as_ref().file_stem().unwrap())
                .join(path.file_stem().unwrap());
            println!("{:?}", out_path);
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

fn subs_file<T, O>(template: T, output: O, subs: &[(String, String)]) -> Result<(), DynError>
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
