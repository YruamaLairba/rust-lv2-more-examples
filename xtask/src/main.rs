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

struct Config {
    subcommand: Option<String>,
    target: Option<String>,
    release: bool,
    packages: Vec<String>,
}

impl Config {
    fn _new() -> Self {
        Self {
            subcommand: None,
            target: None,
            release: false,
            packages: vec![],
        }
    }

    //build config from environment variables
    fn _vars(mut self) -> Self {
        //let vars = env::vars();
        //for (key, value) in vars {
        //    match key {}
        //}
        self
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

    //build config from argument passed to app
    fn _args(mut self) -> Result<Self, DynError> {
        let mut args = env::args();
        self.subcommand = args.nth(1) ;
        while let Some(arg) = args.next() {
                match arg.as_ref() {
                    "-p" | "--package" => self._arg_package(args.next())?,
                    "--release" => self.release = true,
                    "--target" => self._arg_target(args.next())?,
                    _ => return Err(format!("Unexptected argument: '{}'",arg).into())
                }
        }
        Ok(self)
    }
    //build config from environment variable and passed argument
    fn from_env() -> Result<Self, DynError> {
        Config::_new()._vars()._args()
    }
}



struct Package {
    name: String,
    dir: String,                 //relative from the workspace root
    template_files: Vec<String>, //path relative to package dir
    template_subs: Vec<(String, String)>,
}

impl Package {
    fn default(name: &str) -> Self {
        Self {
            name: String::from(name),
            dir: String::from(name),
            template_files: vec![],
            template_subs: vec![],
        }
    }
}

//const eg_worker_rs: Package = Package {
//    name: "eg-worker-rs",
//    dir: "eg-worker-rs",
//    template_files: &["worker.ttl.in","manifest.ttl.in"],
//    template_subs:&[("@LIB_NAME@","eg-worker-rs")],
//};

fn package_list() -> Vec<Package> {
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
    let mut eg_worker_rs = Package::default("eg-worker-rs");
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

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), DynError> {
    let conf = Config::from_env()?;
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
    let mut target = "";
    let mut target_dir = "target";
    let mut profile_dir = "debug";
    //cargo option other than packages
    let mut build_opt = Vec::<String>::new();
    let mut packages = Vec::<String>::new();
    let mut args_iter = args.iter();
    while let Some(e) = args_iter.next() {
        match e.as_ref() {
            "-p" | "--package" => {
                if let Some(p) = args_iter.next() {
                    packages.push(p.clone());
                }
            }
            "--release" => {
                profile_dir = "release";
                build_opt.push(e.clone());
            }
            "--target" => {
                build_opt.push(e.clone());
                if let Some(tg) = args_iter.next() {
                    target = &tg;
                    build_opt.push(tg.clone());
                }
            }
            _ => (),
        }
    }
    let build_path = PathBuf::new()
        .join(workspace_root())
        .join(target_dir)
        .join(target)
        .join(profile_dir);
    println!("target: {}", target);
    println!("target_dir: {}", target_dir);
    println!("profile_dir: {}", profile_dir);
    println!("build_path: {}", build_path.to_string_lossy());
    println!("{:?}", packages);
    if packages.is_empty() {
        for p in package_list() {
            build_template(&p, &build_path);
        }
    }

    println!();

    Ok(())
}

fn build_template<B: AsRef<Path>>(project: &Package, build_path: B) {
    let project_dir = workspace_root().join(&project.dir);
    let out_dir = build_path.as_ref().join("lv2").join(&project.dir);
    for file in &project.template_files {
        let file_path = project_dir.join(&file);
        let file_stem = AsRef::<Path>::as_ref(&file).file_stem().unwrap();
        let out_path = out_dir.join(file_stem);
        subs_file(file_path, out_path, &project.template_subs).unwrap();
    }
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
