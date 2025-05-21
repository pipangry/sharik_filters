use std::io::BufWriter;
use std::path::PathBuf;
use rayon::prelude::*;
use serde::Serialize;
use serde_json::{Serializer, Value};
use walkdir::WalkDir;

fn main() {
    let env_args: Vec<String> = std::env::args().collect();
    let Some(target_dir) = env_args.get(1) else {
        println!("Target directory not specified");
        return;
    };
    
    let indent = match env_args.get(2) {
        Some(indent) => {
            if let Ok(indent) = indent.parse::<u32>() {
                indent
            } else {
                println!("Indentation must be an u32");
                return;
            }
        }
        None => 4,
    };

    let paths = WalkDir::new(target_dir)
        .into_iter()
        .filter_map(|e| { e.ok() })
        .map(|e| e.path().to_owned())
        .filter(|p| {
            p.is_file() && p
                .extension()
                .map(|e| (e == "json") | (e == "jsonc"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    paths.into_par_iter().for_each(|p| {
        if let Err(e) = format_file(p, indent) {
            println!("{}", e)
        }
    })
}

fn format_file(path: PathBuf, indent: u32) -> Result<(), String> {
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Can not read file: {}", e))?;

    let formatted_content = remove_jsonc_comments(&contents);
    let value: Value = serde_json::from_str(&formatted_content)
        .map_err(|e| format!("Malformed json: {} at file {:?}", e, formatted_content))?;

    let file = std::fs::File::create(path)
        .map_err(|e| format!("Unable to create a file: {}", e))?;
    let writer = BufWriter::new(file);

    let indent = " ".repeat(indent as usize);
    let ser_fmt = serde_json::ser::PrettyFormatter::with_indent(indent.as_bytes());
    let mut ser = Serializer::with_formatter(writer, ser_fmt);
    value.serialize(&mut ser).unwrap();
    Ok(())
}

enum CommentsCleanerState {
    Default,
    SingleLine,
    MultiLine,
}

pub fn remove_jsonc_comments(input: &str) -> String {
    let mut chars = input.chars().peekable();
    let mut result = String::with_capacity(input.len());
    let mut in_quotes = false;
    let mut state = CommentsCleanerState::Default;

    while let Some(c) = chars.next() {
        if c == '"' { in_quotes = !in_quotes }
        if in_quotes {
            result.push(c);
            continue;
        }

        match state {
            CommentsCleanerState::Default => {
                if c == '/' {
                    match chars.peek() {
                        Some('/') => {
                            chars.next();
                            state = CommentsCleanerState::SingleLine;
                        }
                        Some('*') => {
                            chars.next();
                            state = CommentsCleanerState::MultiLine;
                        }
                        _ => result.push(c),
                    }
                } else {
                    result.push(c);
                }
            }
            CommentsCleanerState::SingleLine => {
                if c == '\n' {
                    result.push(c);
                    state = CommentsCleanerState::Default;
                }
            }
            CommentsCleanerState::MultiLine => {
                if c == '*' {
                    if let Some('/') = chars.peek() {
                        chars.next();
                        state = CommentsCleanerState::Default;
                    }
                }
            }
        }
    }
    result.shrink_to_fit();
    result
}