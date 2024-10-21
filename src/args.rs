/*
 * Copyright (C) 2024 taylor.fish <contact@taylor.fish>
 *
 * This file is part of fix-ardour-lv2-index.
 *
 * fix-ardour-lv2-index is free software: you can redistribute it and/or
 * modify it under the terms of the GNU General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * fix-ardour-lv2-index is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
 * General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with fix-ardour-lv2-index. If not, see <https://www.gnu.org/licenses/>.
 */

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display};
use std::ops::ControlFlow::{self, Break};
use std::path::PathBuf;

pub const USAGE: &str = "\
[options] <session-file>

Fixes parameter indices in the .ardour file <session-file> and saves
a backup of the original session in <session-file>.orig.

Options:
  -o <file>   Write to <file> instead of modifying the session in-place
  -h, --help  Show this help message
";

#[derive(Debug)]
pub enum Input {
    Stdin,
    Path(PathBuf),
}

#[derive(Debug)]
pub enum Output {
    InPlace,
    Stdout,
    Path(PathBuf),
}

#[derive(Debug)]
pub struct RunArgs {
    pub input: Input,
    pub output: Output,
}

#[derive(Debug)]
pub enum Args {
    Help,
    Run(RunArgs),
}

#[derive(Debug)]
pub enum ArgsError {
    MissingArg,
    UnexpectedArg(OsString),
    BadOption(OsString),
    BadShortOption(char),
    IncompleteOption(&'static str),
    DuplicateOption(&'static str),
}

impl Display for ArgsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArg => write!(f, "missing argument"),
            Self::UnexpectedArg(s) => {
                write!(f, "unexpected argument: {}", s.to_string_lossy())
            }
            Self::BadOption(s) => {
                write!(f, "unknown option: {}", s.to_string_lossy())
            }
            Self::BadShortOption(c) => write!(f, "unknown option: -{c}"),
            Self::IncompleteOption(s) => {
                write!(f, "missing argument for option {s}")
            }
            Self::DuplicateOption(s) => write!(f, "duplicate option: {s}"),
        }
    }
}

struct Parser<A> {
    args: A,
    options_done: bool,
    input: Option<Input>,
    output: Output,
}

impl<A: Iterator<Item = OsString>> Parser<A> {
    fn rest_or_next<'a>(&mut self, rest: &'a OsStr) -> Option<Cow<'a, OsStr>> {
        if rest.is_empty() {
            self.args.next().map(Into::into)
        } else {
            Some(rest.into())
        }
    }

    fn short(
        &mut self,
        opt: char,
        rest: &OsStr,
    ) -> Result<ControlFlow<Option<Args>>, ArgsError> {
        match opt {
            'h' => Ok(Break(Some(Args::Help))),
            'o' => {
                if !matches!(self.output, Output::InPlace) {
                    return Err(ArgsError::DuplicateOption("-o"));
                }
                let Some(next) = self.rest_or_next(rest) else {
                    return Err(ArgsError::IncompleteOption("-o"));
                };
                self.output = match next.as_encoded_bytes() {
                    b"-" => Output::Stdout,
                    _ => Output::Path(next.into_owned().into()),
                };
                Ok(Break(None))
            }
            _ => Err(ArgsError::BadShortOption(opt)),
        }
    }

    fn arg(&mut self, arg: OsString) -> Result<Option<Args>, ArgsError> {
        let bytes = arg.as_encoded_bytes();
        if self.options_done || arg == "-" {
        } else if arg == "--" {
            self.options_done = true;
        } else if arg == "--help" {
            return Ok(Some(Args::Help));
        } else if bytes.starts_with(b"--") {
            return Err(ArgsError::BadOption(arg));
        } else if let Some(mut opts) = bytes.strip_prefix(b"-") {
            while let Some((&opt, rest)) = opts.split_first() {
                opts = rest;
                if !opt.is_ascii() {
                    return Err(ArgsError::BadOption(arg));
                }
                // SAFETY: `rest` starts immediately after an ASCII byte (which
                // is necessarily valid UTF-8) and ends at the end of `arg`, a
                // valid `OsStr`.
                match self.short(opt.into(), unsafe {
                    OsStr::from_encoded_bytes_unchecked(rest)
                })? {
                    ControlFlow::Break(Some(args)) => return Ok(Some(args)),
                    ControlFlow::Break(None) => break,
                    ControlFlow::Continue(()) => {}
                }
            }
            return Ok(None);
        }
        if self.input.is_some() {
            return Err(ArgsError::UnexpectedArg(arg));
        }
        self.input = Some(match bytes {
            b"-" if !self.options_done => Input::Stdin,
            _ => Input::Path(arg.into()),
        });
        Ok(None)
    }

    fn parse(mut self) -> Result<Args, ArgsError> {
        while let Some(arg) = self.args.next() {
            if let Some(args) = self.arg(arg)? {
                return Ok(args);
            }
        }
        let Some(input) = self.input else {
            return Err(ArgsError::MissingArg);
        };
        let output = match (&input, self.output) {
            (Input::Stdin, Output::InPlace) => Output::Stdout,
            (_, out) => out,
        };
        Ok(Args::Run(RunArgs {
            input,
            output,
        }))
    }
}

pub fn parse<A>(args: A) -> Result<Args, ArgsError>
where
    A: IntoIterator<Item = OsString>,
{
    Parser {
        args: args.into_iter(),
        options_done: false,
        input: None,
        output: Output::InPlace,
    }
    .parse()
}
