use std::collections::HashMap;
use std::env;
use std::fs::{self, read_dir};
use std::io::Write;
use std::path::{PathBuf, MAIN_SEPARATOR};
use std::process::Command;
use std::str::Chars;
use std::time::SystemTime;

use which::which;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn main() {
    let mut args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let first = args[1].as_str();
        match first {
            "run" => {
                template(&args, false, "");
                cargo(&args);
            }
            "check" => {
                template(&args, false, "");
                cargo(&args);
            }
            "force-check" => {
                template(&args, true, "");
                args[1] = "check".to_string();
                cargo(&args);
            }
            "new" => {
                cargo(&args);
                new(&args);
            }
            "app" => {
                if args.len() > 2 {
                    app(&args);
                } else {
                    eprintln!("Expected name");
                }
            }
            "migrate" => {
                cargo_run(&mut args);
            }
            "make-view" => {
                make_view(&mut args);
            }
            "sql-migrate" => {
                cargo_run(&mut args);
            }
            "make-migrations" => {
                cargo_run(&mut args);
            }
            "admin" => {
                cargo_run(&mut args);
            }
            "--version" => println!("ananc {}", VERSION),
            _ => usage(),
        }
    }
}

macro_rules! cp {
    ($name:expr, $file:expr) => {
        fs::write(format!("{}/{}", $name.to_owned(), $file), include_bytes!(concat!("skeleton/", $file))).expect("Error copying file");
    };
    ($name:expr, $($file:expr),*) => {
        $(cp!($name, $file);)*
    };
}

macro_rules! cp_as {
    ($name:expr, $file:expr) => {
        fs::write(
            $name.to_owned(),
            include_bytes!(concat!("skeleton/", $file)),
        )
        .expect("Error copying file");
    };
}

fn uppercase(s: &str) -> String {
    let mut c = s.chars();
    c.next().unwrap().to_uppercase().collect::<String>() + c.as_str()
}

fn make_view(args: &Vec<String>) {
    for arg in &args[2..] {
        let mview = format!("{}", arg);
        fs::create_dir(&mview).expect("Failed to create path");
        let temp = format!("{}/templates", arg);
        fs::create_dir(&temp).expect("Failed to create path");
        let parsed = format!(
            "{}/templates/.parsed",
            arg
        );
        fs::create_dir(&parsed).expect("Failed to create path");
        let upper = uppercase(arg);
        make_file(arg, "views", ".rs", format!("use crate::prelude::*;\nuse super::super::records::{{{}}};\n\n#[base_view]\nfn base<R: Request>(_req: R) -> Result<Response> {{}}\n\n#[record_view]\nimpl<R: Request> {0}View<R> {{\n    #[view(Group::is_visitor)]\n    pub async fn index(req: R) -> Result<Response> {{\n        let title = \"Title\";\n    }}\n}}", upper));
        make_file(arg, "mod", ".rs", "pub mod views;".to_string());
        cp_as!(
            format!("{}/index.rs.html", temp),
            "templates/index.rs.html"
        );
        cp_as!(
            format!("{}/base.rs.html", temp),
            "templates/base.rs.html"
        );
        append(".", "mod.rs", &format!("pub mod {};\n", arg).into_bytes());
        println!("Created view \"{}\"", arg);
    }
}

fn usage() {
    eprintln!("Anansi's project manager\n\nUSAGE:\n    ananc [OPTIONS] [SUBCOMMAND]\n\nOPTIONS:\n    --version\tPrint version info and exit\n\nIn addition to Cargo's commands, some others are:\n    app\t\t\tCreate an app\n    sql-migrate\t\tView SQL for migration files\n    make-migrations\tCreate migration files for the project\n    migrate\t\tApply migrations");
}

fn new(args: &Vec<String>) {
    let name = &args[2];
    let src = &format!("{}/src/", args[2]);
    cp!(args[2], "settings.toml");
    cp!(src, "project.rs", "urls.rs", "main.rs");
    make_file(&name, ".gitignore", "", "".to_string());
    append(name, ".gitignore", b"/settings.toml\n/database.db*");
    append(
        name,
        "Cargo.toml",
        &format!("anansi = \"{}\"\nasync-trait = \"0.1.57\"", VERSION).into_bytes(),
    );
    fs::create_dir(format!("{}/src/http_errors", name)).unwrap();
    cp!(src, "http_errors/500.html");
    cp!(src, "http_errors/views.rs");
    cp!(src, "http_errors/mod.rs");
    fs::create_dir(format!("{}/http_errors/templates", src)).unwrap();
    cp!(src, "http_errors/templates/not_found.rs.html");
    fs::create_dir(format!("{}/http_errors/templates/.parsed", src)).unwrap();
    template(args, false, &format!("/{name}"));
}

fn append(dir_name: &str, file_name: &str, content: &[u8]) {
    let path = format!("{}/{}", dir_name, file_name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .open(&path)
        .expect(&format!("error opening {}", &path));
    file.write_all(content).unwrap();
}

fn app(args: &Vec<String>) {
    let name = &args[2];
    fs::create_dir(name).expect("Failed to create app directory");
    fs::create_dir(&format!("{}/migrations", name)).expect("Failed to create migrations directory");
    make(
        name,
        "init",
        format!("pub const APP_NAME: &'static str = \"{}\";", name),
    );
    make(
        name,
        "mod",
        "pub mod init;\npub mod urls;\npub mod records;\npub mod migrations;\n".to_string(),
    );
    make(
        name,
        "urls",
        "use anansi::web::prelude::*;\n\nroutes! {}".to_string(),
    );
    make(name, "migrations/mod", "pub mod init;".to_string());
    make(
        name,
        "migrations/init",
        "use anansi::migrations::prelude::*;\n\nlocal_migrations! {}".to_string(),
    );
    cp!(name, "records.rs");
    println!("Created app \"{name}\"");
}

fn make_file(dir: &str, name: &str, ext: &str, content: String) {
    let path = format!("{}/{}{}", dir, name, ext);
    fs::write(&path, content).unwrap();
}

fn make(dir: &str, name: &str, content: String) {
    make_file(dir, name, ".rs", content);
}

fn template(args: &Vec<String>, mut force: bool, extra: &str) {
    let path = match fs::canonicalize(&args[0]) {
        Ok(o) => o,
        Err(_) => which(&args[0]).unwrap(),
    };
    let date = fs::metadata(path).unwrap().modified().unwrap();
    let cwd = env::current_dir().unwrap();
    let cs = cwd.clone().into_os_string().into_string().unwrap() + extra;
    let src = format!("/src");
    let dir = match cs.rfind(&src) {
        Some(index) => {
            let s = std::str::from_utf8(&cs.as_bytes()[..index])
                .unwrap()
                .to_string()
                + &src;
            PathBuf::from(s)
        }
        None => PathBuf::from(cs),
    };
    search(&date, dir, &mut force);
}

fn search(date: &SystemTime, current: PathBuf, force: &mut bool) {
    let dirs = read_dir(current).unwrap();
    for f in dirs {
        let f = f.unwrap();
        if f.file_type().unwrap().is_dir() {
            if f.file_name() == "templates" {
                let parent = f.path().into_os_string().into_string().unwrap();
                let dirs = read_dir(f.path()).unwrap();
                for f in dirs {
                    let f = f.unwrap();
                    let name = f.path().into_os_string().into_string().unwrap();
                    if name.ends_with(".rs.html") {
                        let b = check_template(f, &parent, name, *date, *force);
                        if b {
                            *force = true;
                        }
                    } else if f.file_type().unwrap().is_dir() {
                        let parent = f.path().into_os_string().into_string().unwrap();
                        let dirs = read_dir(f.path()).unwrap();
                        for f in dirs {
                            let f = f.unwrap();
                            let name = f.path().into_os_string().into_string().unwrap();
                            if name.ends_with(".rs.html") {
                                check_template(f, &parent, name, *date, *force);
                            }
                        }
                    }
                }
            } else {
                search(date, f.path(), force);
            }
        }
    }
}

fn check_template(
    f: std::fs::DirEntry,
    parent: &String,
    name: String,
    date: std::time::SystemTime,
    force: bool,
) -> bool {
    let template = f.metadata().unwrap().modified().unwrap();
    let n = f.file_name().into_string().unwrap();
    let (n, _) = n.split_once('.').unwrap();
    let p = format!("{}/.parsed/{}.in", parent, n);
    let prs = format!("{}/.parsed", parent);
    let mut parser = Parser::new();
    if std::path::Path::new(&prs).exists() {
        let parsed = fs::metadata(p);
        if parsed.is_err() {
            parser.parse(&name);
        } else {
            let modified = parsed.unwrap().modified().unwrap();
            if force || template > modified || modified < date {
                parser.parse(&name);
                return true;
            }
        }
    } else {
        eprintln!("{} was not parsed", name);
    }
    false
}

fn cargo_run(args: &mut Vec<String>) {
    args.insert(1, "run".to_string());
    cargo(&args);
}

fn cargo(args: &Vec<String>) {
    let mut cmd = Command::new("cargo");
    for arg in &args[1..] {
        cmd.arg(arg);
    }
    let mut child = cmd.spawn().expect("Failed to start cargo");
    child.wait().expect("failed to wait on child");
}

struct Parser {
    blocks: Vec<String>,
}

impl Parser {
    fn new() -> Self {
        Self { blocks: vec![] }
    }
    fn parse(&mut self, name: &str) {
        let content = fs::read_to_string(name).unwrap();
        let mut chrs = content.chars();
        let e = collect(&mut chrs, ' ');
        let (dir, temp) = name.rsplit_once(MAIN_SEPARATOR).unwrap();
        let mut base = false;
        let view = if e == "@extend" {
            self.extend(&mut chrs)
        } else if e == "@base_extend" {
            base = true;
            self.extend(&mut chrs)
        } else {
            let mut view = String::from("{let mut _c = String::new();");
            view.push_str(&self.process(content));
            view.push_str("Ok(anansi::web::Response::new(\"HTTP/1.1 200 OK\", _c.into_bytes()))}");
            view
        };
        let out = &temp[..temp.find('.').unwrap()];

        if !self.blocks.is_empty() || base {
            let mut s = "pub struct Args {".to_string();
            for block in &self.blocks {
                s.push_str(&format!("pub _{}: String,", block));
            }
            s.push_str("}");
            let mut f = fs::File::create(format!(
                "{}/.parsed/{}_args.in",
                dir, out
            ))
            .unwrap();
            write!(f, "{}", s).unwrap();
        }

        let mut f = fs::File::create(format!(
            "{}/.parsed/{}.in",
            dir, out
        ))
        .unwrap();
        write!(f, "{}", view).unwrap();
    }
    fn process(&mut self, content: String) -> String {
        let mut view = String::from("_c.push_str(\"");
        let mut chars = content.chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    view.push_str("\\\"");
                }
                '\\' => {
                    view.push_str("\\\\");
                }
                '@' => {
                    self.at(&mut view, &mut chars);
                }
                _ => {
                    view.push(c);
                }
            }
        }
        view.push_str("\");");
        view
    }
    fn extend(&mut self, chars: &mut Chars) -> String {
        let basename = collect(chars, '\n').trim().to_string();
        let mut blocks = HashMap::new();
        loop {
            skip(chars, '@');
            let s = collect(chars, ' ');
            if s == "block" {
                let name = collect(chars, ' ');
                skip(chars, '{');
                let mut block = String::new();
                if let Some(c) = chars.next() {
                    if c != '\n' {
                        block.push(c);
                    }
                }
                get_block(chars, &mut block);
                let block = self.process(block);
                let block = format!("{{let mut _c = String::new();{} _c}}", block);
                blocks.insert(name, block);
            } else {
                break;
            }
        }
        let args = format!("{}::Args", basename);
        let mut ext = format!("{}::base(req, {}{{", basename, args);
        let mut s = String::from("{");
        for (name, src) in blocks {
            s.push_str(&format!("let _{} = {{{}}};", name, src));
            ext.push_str(&format!("_{}, ", name));
        }
        s.push_str(&ext);
        s.push_str("})}");
        s
    }
}

fn skip(chars: &mut Chars, chr: char) {
    while let Some(c) = chars.next() {
        if c == chr {
            break;
        }
    }
}

fn collect_paren(chars: &mut Chars) -> String {
    let mut s = String::new();
    let mut n = 0;
    let mut quote = false;
    while let Some(c) = chars.next() {
        s.push(c);
        if !quote {
            match c {
                ')' => {
                    if n == 0 {
                        break;
                    }
                    n -= 1;
                }
                '(' => {
                    n += 1;
                }
                '"' => {
                    quote = true;
                }
                _ => {}
            }
        } else {
            match c {
                '\'' => {
                    if let Some(d) = chars.next() {
                        s.push(d);
                    } else {
                        break;
                    }
                }
                '"' => {
                    quote = false;
                }
                _ => {}
            }
        }
    }
    s
}

fn collect(chars: &mut Chars, chr: char) -> String {
    let mut s = String::new();
    let mut take = chars.take_while(|c| *c != chr);
    while let Some(c) = take.next() {
        s.push(c);
    }
    s
}

fn collect_name(chars: &mut Chars) -> (String, char) {
    let mut name = String::new();
    loop {
        if let Some(c) = chars.next() {
            match c {
                ' ' => {}
                '<' => {}
                '\n' => {}
                _ => {
                    name.push(c);
                    continue;
                }
            }
            break (name, c);
        }
        panic!("error collecting name");
    }
}

fn get_block(chars: &mut Chars, block: &mut String) {
    let mut n = 0;
    let mut quote = false;
    while let Some(c) = chars.next() {
        if c == '\\' {
            block.push(c);
            if let Some(d) = chars.next() {
                block.push(d);
            }
        } else {
            if !quote {
                match c {
                    '{' => n += 1,
                    '}' => {
                        if n == 0 {
                            return;
                        }
                        n -= 1;
                    }
                    '"' => quote = true,
                    '\n' => {
                        if let Some(d) = chars.next() {
                            if d == '}' {
                                if n == 0 {
                                    return;
                                } else {
                                    n -= 1;
                                }
                            }
                            block.push(c);
                            block.push(d);
                            continue;
                        }
                    }
                    _ => {}
                }
            } else {
                if c == '"' {
                    quote = false;
                }
            }
            block.push(c);
        }
    }
}

impl Parser {
    fn at(&mut self, view: &mut String, chars: &mut Chars) {
        view.push_str("\");");
        let mut s = String::new();
        let mut extra = String::new();
        if let Some(c) = chars.next() {
            if c == '(' {
                let mut p = 0;
                while let Some(d) = chars.next() {
                    if d == '(' {
                        p += 1;
                    } else if d == ')' {
                        if p == 0 {
                            break;
                        } else {
                            p -= 1;
                        }
                    }
                    s.push(d);
                }
                view.push_str(&format!(
                    "_c.push_str(&anansi::web::html_escape(&format!(\"{{}}\", {})));_c.push_str(\"",
                    s
                ));
                return;
            }
            s.push(c);
        } else {
            return;
        }
        while let Some(c) = chars.next() {
            if c == ' ' || c == '<' || c == '\n' || c == '(' || c == MAIN_SEPARATOR {
                extra.push(c);
                break;
            } else if c == '"' {
                extra.push_str("\\\"");
                break;
            }
            s.push(c);
        }
        let keyword = s.clone();
        let mut find_brace = true;
        match s.as_str() {
            "if" => {}
            "for" => {}
            "loop" => {}
            "while" => {}
            "block" => {
                let (name, ex) = collect_name(chars);
                self.blocks.push(name.clone());
                view.push_str(&format!(
                    "_c.push_str(&_base_args._{});_c.push_str(\"{}",
                    name, ex
                ));
                return;
            }
            "build" => {
                let (name, ex) = collect_name(chars);
                s = format!("_c.push_str(&{}.tag()); if let Some(token_tag) = form.token_tag() {{ _c.push_str(&token_tag) }}", name);
                if ex == '{' {
                    find_brace = false;
                }
            }
            "unescape" => {
                let (name, ex) = collect_name(chars);
                view.push_str(&format!(
                    "_c.push_str(&format!(\"{{}}\", {}));_c.push_str(\"{}",
                    name, ex
                ));
                return;
            }
            "link" => {
                s.clear();
                let line = collect(chars, '{');
                let args: Vec<&str> = line.split(',').collect();
                let mut segments = vec![];
                let mut attrs = String::new();
                for arg in args {
                    if arg.contains("=") {
                        attrs.push_str(&format!(" {}", arg.replace("\"", "\\\"")));
                    } else {
                        segments.push(arg);
                    }
                }
                let mut u = String::from(segments[0].clone());
                for segment in &segments[1..] {
                    u.push_str(&format!(", {}", segment));
                }
                view.push_str(&format!(
                    "_c.push_str(&format!(\"<a href=\\\"{{}}\\\"{}>\", anansi::url!({})));",
                    attrs, u
                ));
                let blk = collect(chars, '}');
                view.push_str(&self.process(blk));
                return view.push_str("_c.push_str(\"</a>");
            }
            "url!" => {
                s = "anansi::url!".to_string();
                variable(&extra, &mut s, chars, view);
                return;
            }
            _ => {
                let mut c = s.chars();
                if c.next().unwrap() == '{' {
                    let r: String = c.collect();
                    let blk = collect(chars, '}');
                    view.push_str(&r);
                    view.push(' ');
                    view.push_str(&blk);
                    view.push_str("_c.push_str(\"");
                } else {
                    variable(&extra, &mut s, chars, view);
                }
                return;
            }
        }
        s.push_str(&extra);
        if find_brace {
            while let Some(c) = chars.next() {
                s.push(c);
                if c == '{' {
                    break;
                }
            }
        }
        view.push_str(&format!("{}_c.push_str(\"", s));
        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    view.push_str("\\\"");
                }
                '\\' => {
                    view.push_str("\\\\");
                    if let Some(d) = chars.next() {
                        view.push(d);
                    }
                }
                '}' => {
                    match keyword.as_str() {
                        "build" => view.push_str("</form>"),
                        _ => {}
                    }
                    view.push_str("\");}_c.push_str(\"");
                    return;
                }
                '@' => {
                    self.at(view, chars);
                }
                _ => {
                    view.push(c);
                }
            }
        }
        view.push_str("_c.push_str(\"");
    }
}

fn variable(extra: &str, s: &mut String, chars: &mut Chars, view: &mut String) {
    if extra == "(" {
        s.push_str(extra);
        s.push_str(&collect_paren(chars));
        view.push_str(&format!(
            "_c.push_str(&anansi::web::html_escape(&format!(\"{{}}\", {})));_c.push_str(\"",
            s
        ));
    } else {
        view.push_str(&format!(
            "_c.push_str(&anansi::web::html_escape(&format!(\"{{}}\", {})));_c.push_str(\"{}",
            s, extra
        ));
    }
}
