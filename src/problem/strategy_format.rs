use crate::problem::strategy;
use anyhow::{bail, Context, Result};
use regex::Regex;
use std::collections::HashMap;

pub struct ParsedStrategy {
    pub files: HashMap<String, strategy::FileType>,
    pub blocks: Vec<strategy::Block>,
}

pub fn parse_sunwalker_strategy(file: &str) -> Result<ParsedStrategy> {
    let mut current_block = None;

    let mut strategy_format = ParsedStrategy {
        files: HashMap::new(),
        blocks: Vec::new(),
    };

    let mut commit_current_block = || -> Result<()> {
        if let Some((name, lines)) = current_block.take() {
            strategy_format
                .blocks
                .push(parse_block(name, lines).with_context(|| format!("In block {name}"))?);
        }
        Ok(())
    };

    for line in file.lines() {
        if line.is_empty() {
            continue;
        }
        if line.chars().next().unwrap().is_whitespace() {
            current_block
                .as_mut()
                .context("Strategy file cannot contain whitespace-indented code outside blocks")?
                .1
                .push(line.trim_start());
        } else {
            // Global configuration or start of new block
            let tokens = split_tokens(line, true)?;
            let mut it = tokens.into_iter();

            let Token::String(command) = it.next().unwrap() else {
                bail!("Each line outside blocks must start with a directive, which is a normal identifier, not a redirect or a filename");
            };

            match command.as_ref() {
                "file" | "pipe" => {
                    if current_block.is_some() || !strategy_format.blocks.is_empty() {
                        bail!("Directive '{command}' must appear before blocks");
                    }
                    for token in it {
                        let Token::File(filename) = token else {
                            bail!(
                                "Directive '{command}' must be followed by filenames, each of \
                                 which starts with %"
                            );
                        };
                        if strategy_format
                            .files
                            .insert(
                                filename.to_string(),
                                match command.as_ref() {
                                    "file" => strategy::FileType::Regular,
                                    "pipe" => strategy::FileType::Pipe,
                                    _ => unreachable!(),
                                },
                            )
                            .is_some()
                        {
                            bail!("filename '%{filename}' is defined twice");
                        }
                    }
                }
                "block" => {
                    commit_current_block()?;
                    let name = it
                        .next()
                        .context("Directive 'block' must be followed by a block name")?;
                    let Token::String(name) = it.next().unwrap() else {
                        bail!(
                            "Directive 'block' must be followed by a block name, which is a \
                             normal identifier, not a redirect or a filename"
                        )
                    };
                    if name.is_empty() {
                        bail!(
                            "Directive 'block' must be followed by a non-empty block name. The \
                             empty name is invalid."
                        );
                    }
                    if let Some(text) = it.next() {
                        bail!(
                            "The block name {name:?} in directive 'block' is followed by stray \
                             text"
                        );
                    }
                    current_block = Some((name, Vec::new()));
                }
                _ => {
                    bail!(
                        "Unknown directive '{command}' at the outer level. The supported \
                         directives are 'file', 'pipe', and 'block'."
                    );
                }
            }
        }
    }
    commit_current_block()?;

    Ok(strategy_format)
}

fn parse_block(name: String, lines: Vec<&str>) -> Result<strategy::Block> {
    let mut tactic = None;
    let mut bindings = HashMap::new();

    let shell_command = lines.pop().context("Block contains no content")?;

    for line in lines {
        let tokens = split_tokens(line, true)?;
        let mut it = tokens.into_iter();
        let Token::String(command) = it.next().unwrap() else {
            bail!(
                "Each block line (except the outside blocks one) must start with a directive, \
                 which is a normal identifier, not a redirect or a filename"
            );
        };

        match command.as_ref() {
            "tactic" => {
                let parsed_tactic = it.next().context(
                    "Directive 'tactic' must be followed by a tactic name: 'user' or 'testlib'",
                )?;
                let Token::String(parsed_tactic) = parsed_tactic else {
                    bail!(
                        "Directive 'tactic' must be followed by a tactic name: 'user' or 'testlib'"
                    );
                };
                let parsed_tactic_object = match parsed_tactic.as_ref() {
                    "user" => strategy::Tactic::User,
                    "testlib" => strategy::Tactic::Testlib,
                    _ => {
                        bail!(
                            "Unknown tactic '{parsed_tactic}': the supported tactics are 'user' \
                             and 'testlib'"
                        );
                    }
                };
                if let Some(text) = it.next() {
                    bail!(
                        "The tactic name '{parsed_tactic}' in directive 'tactic' is followed by \
                         stray text"
                    );
                }
                if tactic.replace(parsed_tactic_object).is_some() {
                    bail!("Directive 'tactic' can only appear once per block");
                }
            }
            "ro" | "rw" => {
                let source = it.next().with_context(|| {
                    format!("Directive '{command}' must be followed by a source filename")
                })?;
                let source = match source {
                    Token::String(s) => strategy::Pattern::VariableText(s),
                    Token::File(s) => strategy::Pattern::File(s),
                    _ => {
                        bail!(
                            "Directive '{command}' must be followed by a source filename, which \
                             must be either a filename or normal text"
                        );
                    }
                };

                let as_str = it.next().with_context(|| {
                    format!(
                        "Directive '{command}' must be followed by a source filename, and then by \
                         'as', but EOL was seen"
                    )
                })?;
                let as_word = "as".to_string();
                match as_str {
                    Token::String(as_word) => {}
                    _ => {
                        bail!(
                            "Directive '{command}' must be followed by a source filename, and \
                             then by 'as', but a different token was seen"
                        )
                    }
                };

                let location = it.next().with_context(|| {
                    format!(
                        "The 'as' in directive '{command}' must be followed by a target location"
                    )
                })?;
                let location = match location {
                    Token::String(location) => location,
                    _ => {
                        bail!(
                            "The 'as' in directive '{command}' must be followed by a target \
                             location, which must be a normal string"
                        );
                    }
                };
                if location.is_empty() {
                    bail!(
                        "The 'as' in directive '{command}' must be followed by a non-empty target \
                         location. The empty location is invalid."
                    );
                }

                if bindings
                    .insert(
                        location,
                        strategy::Binding {
                            readable: true,
                            writable: command == "rw",
                            source,
                        },
                    )
                    .is_some()
                {
                    bail!("Target location {location:?} is mapped twice");
                }
            }
            _ => {
                bail!(
                    "Unknown directive '{command}' at block level. The supported directives are \
                     'tactic', 'ro', and 'rw'."
                );
            }
        }
    }

    let mut block = strategy::Block {
        name,
        tactic: tactic.context("Directive 'tactic' is missing")?,
        bindings,
        command: "".to_string(),
        argv: Vec::new(),
        stdin: None,
        stdout: None,
        stderr: None,
    };

    let tokens = split_tokens(shell_command, true).context("Failed to parse shell command")?;
    let mut it = tokens.into_iter();

    for token in it {
        match token {
            Token::Redirect(stream) => {
                let next_token = it.next().with_context(|| {
                    format!("A redirect should be followed by a file path, but EOL was seen")
                })?;

                let file = match next_token {
                    Token::String(s) => strategy::Pattern::VariableText(s),
                    Token::File(file) => strategy::Pattern::File(file),
                    Token::Redirect(_) | Token::RedirectTo(_, _) => {
                        bail!(
                            "A redirect should be followed by a file path, but another redirect \
                             was seen"
                        );
                    }
                };

                let stream_object = match stream {
                    StandardStream::Stdin => &mut block.stdin,
                    StandardStream::Stdout => &mut block.stdout,
                    StandardStream::Stderr => &mut block.stderr,
                };

                *stream_object = Some(file);

                if let strategy::Pattern::VariableText(s) = file {
                    if s == "/dev/null" {
                        *stream_object = None;
                    }
                }
            }
            Token::RedirectTo(stream, stream_to) => {
                let stream_object = match stream {
                    StandardStream::Stdin => &mut block.stdin,
                    StandardStream::Stdout => &mut block.stdout,
                    StandardStream::Stderr => &mut block.stderr,
                };

                *stream_object = match stream {
                    StandardStream::Stdin => &block.stdin,
                    StandardStream::Stdout => &block.stdout,
                    StandardStream::Stderr => &block.stderr,
                }
                .clone();
            }
            Token::String(s) => {
                block.argv.push(strategy::Pattern::VariableText(s));
            }
            Token::File(s) => {
                block.argv.push(strategy::Pattern::File(s));
            }
        }
    }

    if block.argv.is_empty() {
        bail!("Command is missing");
    }

    block.command = match block.argv.remove(0) {
        strategy::Pattern::File(s) => {
            bail!("The command must be a simple identifier, but it is %{s}, which is a filename");
        }
        strategy::Pattern::VariableText(s) => {
            if s.contains('\x00') {
                bail!(
                    "The command must be a simple identifier, but it contains a variable reference"
                );
            }
            s
        }
    };

    Ok(block)
}

enum Token {
    String(String),
    File(String),
    Redirect(StandardStream),
    RedirectTo(StandardStream, StandardStream),
}

enum StandardStream {
    Stdin,
    Stdout,
    Stderr,
}

fn split_tokens(text: &str, parse_complex_types: bool) -> Result<Vec<Token>> {
    let shell_escape = Regex::new(
        r#"(?x)
            \\(?:[0-7]{3}|[abefnrtv'"?\\]|x[0-9a-f]{2}|u[0-9a-f]{4}|U[0-9a-f]{8})
        "#,
    )
    .unwrap();

    let shell_argument_chunk = Regex::new(&if parse_complex_types {
        format!(
            r#"(?x)
                # A single argument is a concatenation of:
                # Quoted strings possibly containing escapes, or
                |"(?P<quoted>(?:(?:{shell_escape})|.)*?)"
                # Variables, or
                |\$(?P<variable>\w+|{{\w+}})
                # Escapes or non-whitespace characters
                (?P<char>(?:{shell_escape})|.)
            "#
        )
    } else {
        format!(
            r#"(?x)
                # A single argument is a concatenation of:
                # Quoted strings possibly containing escapes, or
                |"(?P<quoted>(?:(?:{shell_escape})|.)*?)"
                # Escapes or non-whitespace characters
                (?P<char>(?:{shell_escape})|.)
            "#
        )
    })
    .unwrap();

    let shell_command_re = Regex::new(&if parse_complex_types {
        format!(
            r#"(?x)
                # Parse redirects first: еhe target of the redirect is the next argument, or 'to' if
                # it exists
                |(?:(?P<redirect>\d*[<>])(?:&(?P<to>\d+))?)
                # A filename
                |%(?P<file>\w+)
                # A single argument
                |(?P<argument>(?:{shell_argument_chunk})+)
            "#
        )
    } else {
        format!(
            r#"(?x)
                (?P<argument>(?:{shell_argument_chunk})+)
            "#
        )
    })
    .unwrap();

    let mut tokens = Vec::new();
    for caps in shell_command_re.captures_iter(text) {
        if let Some(redirect) = caps.name("redirect") {
            let redirect = redirect.as_str();
            let is_output = redirect.bytes().last().unwrap() == b'>';
            let fd = &redirect[..redirect.len() - 1];
            let fd: u32 = if fd.is_empty() {
                if is_output {
                    1
                } else {
                    0
                }
            } else {
                fd.parse()
                    .with_context(|| format!("Invalid file descriptor in redirect: {redirect}"))?
            };

            let stream = match (is_output, fd) {
                (false, 0) => StandardStream::Stdin,
                (true, 1) => StandardStream::Stdout,
                (true, 2) => StandardStream::Stderr,
                _ => bail!(
                    "Redirect {fd}{} is not supported due to operating system incompatibilities",
                    if is_output { ">" } else { "<" }
                ),
            };

            if let Some(to) = caps.name("to") {
                let to = to
                    .as_str()
                    .parse()
                    .with_context(|| format!("Invalid file descriptor in redirect: {redirect}"))?;

                let stream_to = match (is_output, to) {
                    (false, 0) => StandardStream::Stdin,
                    (true, 1) => StandardStream::Stdout,
                    (true, 2) => StandardStream::Stderr,
                    _ => bail!(
                        "Redirect {fd}{}&{to} is not supported due to operating system \
                         incompatibilities",
                        if is_output { ">" } else { "<" }
                    ),
                };

                tokens.push(Token::RedirectTo(stream, stream_to));
            } else {
                tokens.push(Token::Redirect(stream));
            }
        } else if let Some(file) = caps.name("file") {
            tokens.push(Token::File(file.as_str().to_string()));
        } else {
            let argument = &caps["argument"];
            let mut s = String::new();
            for caps in shell_argument_chunk.captures_iter(argument) {
                if let Some(quoted) = caps.name("quoted") {
                    s += &parse_escapes(quoted.as_str())?;
                } else if let Some(variable) = caps.name("variable") {
                    let variable = variable.as_str();
                    let variable = variable
                        .strip_prefix("{")
                        .and_then(|var| var.strip_suffix("}"))
                        .unwrap_or(variable);
                    s.push('\0');
                    s += variable;
                    s.push('\0');
                } else {
                    s += &parse_escapes(&caps["char"])?;
                }
            }

            tokens.push(Token::String(s));
        }
    }

    Ok(tokens)
}

fn parse_escapes(text: &str) -> Result<String> {
    let shell_escape = Regex::new(
        r#"(?x)
            \\([0-3][0-7]{2}|[abefnrtv]|x[0-9a-fA-F]{2}|u[0-9a-fA-F]{4}|U[0-9a-fA-F]{8}|.)
        "#,
    )
    .unwrap();

    let result: Vec<u8> = Vec::new();

    for m in shell_escape.find_iter(text) {
        let s = &m.as_str()[1..];
        match s.chars().nth(0).unwrap() {
            'a' => result.push(b'\x07'),
            'b' => result.push(b'\x08'),
            'e' => result.push(b'\x1b'),
            'f' => result.push(b'\x0c'),
            'n' => result.push(b'\n'),
            'r' => result.push(b'\r'),
            't' => result.push(b'\t'),
            'v' => result.push(b'\x0b'),
            'x' => result.push(u8::from_str_radix(&s[1..], 16).unwrap()),
            'u' | 'U' => result.extend(
                char::from_u32(u32::from_str_radix(&s[1..], 16).unwrap())
                    .with_context(|| format!("{s} is not a valid unicode character"))?
                    .to_string()
                    .as_bytes(),
            ),
            c => result.extend(c.to_string().as_bytes()),
        }
    }

    let result = String::from_utf8(result).context("Invalid UTF-8 string {text:?}")?;

    if result.contains('\0') {
        bail!("Strings must not contain null characters");
    }

    Ok(result)
}

pub fn encode_string(s: &str) -> String {
    // TODO: is this correct?
    format!("{s:?}")
}
