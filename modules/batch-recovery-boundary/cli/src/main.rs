use clap::{Parser, Subcommand};
use memoria_memory::{
    ContentHash, ContextOptions, ExpectedContent, MemoryRepo, Mutation, RepoError,
};
use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "memoria", about = "Durable Markdown memory for agents")]
struct Args {
    #[arg(long, global = true)]
    repo: Option<PathBuf>,
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Init {
        directory: PathBuf,
    },
    Read {
        path: String,
    },
    Write {
        path: String,
        #[arg(long)]
        expect: String,
    },
    Delete {
        path: String,
        #[arg(long)]
        expect: String,
    },
    Apply {
        batch: PathBuf,
    },
    Tree {
        #[arg(long)]
        descriptions: bool,
    },
    Context {
        #[arg(long, default_value_t = 32768)]
        max_bytes: usize,
    },
    Status,
    Diff,
    Commit {
        message: String,
    },
    Log,
}

fn repository_path(override_path: Option<PathBuf>) -> Result<PathBuf, RepoError> {
    if let Some(path) = override_path {
        return Ok(path);
    }
    let cwd = std::env::current_dir().map_err(RepoError::Io)?;
    let repository = git2_discover(&cwd)?;
    Ok(repository)
}

fn git2_discover(cwd: &std::path::Path) -> Result<PathBuf, RepoError> {
    let repository = git2::Repository::discover(cwd)?;
    repository
        .workdir()
        .map(PathBuf::from)
        .ok_or(RepoError::UnsupportedRepository)
}

fn expected(raw: &str) -> Result<ExpectedContent, RepoError> {
    if raw == "absent" {
        Ok(ExpectedContent::Absent)
    } else {
        Ok(ExpectedContent::Hash(
            ContentHash::new(raw).map_err(RepoError::Memory)?,
        ))
    }
}

fn print_result(value: Value, json_mode: bool) -> Result<(), RepoError> {
    let rendered = if json_mode {
        json!({"schema_version":1,"data":value}).to_string()
    } else if let Some(text) = value.as_str() {
        text.to_owned()
    } else {
        serde_json::to_string_pretty(&value)
            .map_err(|_| RepoError::InvalidRequest("output encoding".into()))?
    };
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(rendered.as_bytes())
        .map_err(RepoError::Io)?;
    stdout.write_all(b"\n").map_err(RepoError::Io)
}

fn run(args: Args) -> Result<(), RepoError> {
    let json_mode = args.json;
    let value = match args.command {
        Command::Init { directory } => {
            MemoryRepo::init(&directory)?;
            json!({"repository":directory})
        }
        command => {
            let repository = MemoryRepo::open(repository_path(args.repo)?)?;
            match command {
                Command::Read { path } => {
                    let file = repository.read(&path)?;
                    json!({"path":path,"content":file.content(),"hash":file.hash().as_str(),"metadata":file.metadata()})
                }
                Command::Write { path, expect } => {
                    let mut content = String::new();
                    io::stdin().read_to_string(&mut content).map_err(|error| {
                        if error.kind() == io::ErrorKind::InvalidData {
                            RepoError::InvalidRequest("stdin must be UTF-8".into())
                        } else {
                            RepoError::Io(error)
                        }
                    })?;
                    let receipt = repository.write(&path, &content, expected(&expect)?)?;
                    json!({"operation_id":receipt.operation_id()})
                }
                Command::Delete { path, expect } => {
                    let hash = match expected(&expect)? {
                        ExpectedContent::Hash(hash) => hash,
                        ExpectedContent::Absent => {
                            return Err(RepoError::Memory(memoria_memory::MemoryError::InvalidHash))
                        }
                    };
                    let receipt = repository.delete(&path, hash)?;
                    json!({"operation_id":receipt.operation_id()})
                }
                Command::Apply { batch } => {
                    let mut bytes = Vec::new();
                    std::fs::File::open(batch)
                        .map_err(RepoError::Io)?
                        .take(64 * 1024 * 1024 + 1)
                        .read_to_end(&mut bytes)
                        .map_err(RepoError::Io)?;
                    if bytes.len() > 64 * 1024 * 1024 {
                        return Err(RepoError::LimitExceeded);
                    }
                    let operations: Vec<Mutation> = serde_json::from_slice(&bytes)
                        .map_err(|_| RepoError::InvalidRequest("batch JSON".into()))?;
                    let receipt = repository.apply(operations)?;
                    json!({"operation_id":receipt.operation_id()})
                }
                Command::Tree { descriptions } => json!(repository.tree(descriptions)?),
                Command::Context { max_bytes } => json!(repository
                    .build_context(ContextOptions::new(max_bytes))?
                    .text()),
                Command::Status => json!(repository.status()?),
                Command::Diff => json!(repository.diff()?),
                Command::Commit { message } => json!({"commit":repository.commit(&message)?}),
                Command::Log => json!(repository.log()?),
                Command::Init { .. } => unreachable!(),
            }
        }
    };
    print_result(value, json_mode)?;
    Ok(())
}

fn main() {
    let json_mode = std::env::args_os().any(|argument| argument == "--json");
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                let _ = error.print();
                return;
            }
            if json_mode {
                eprintln!(
                    "{}",
                    json!({"schema_version":1,"error":{"code":"invalid_request","message":error.to_string()}})
                );
            } else {
                eprintln!("{error}");
            }
            std::process::exit(2);
        }
    };
    if let Err(error) = run(args) {
        if json_mode {
            eprintln!(
                "{}",
                json!({"schema_version":1,"error":{"code":error.code(),"message":error.to_string()}})
            );
        } else {
            eprintln!("memoria [{}]: {error}", error.code());
        }
        std::process::exit(1);
    }
}
