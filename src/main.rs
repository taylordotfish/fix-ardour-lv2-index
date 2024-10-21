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

#![warn(clippy::undocumented_unsafe_blocks)]

use std::ffi::OsString;
use std::fmt::{Display, Write as _};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod args;
use args::{Args, USAGE};

mod lv2;
mod patch;
mod session;

fn write_display<P, T>(path: P, contents: &T) -> io::Result<()>
where
    P: AsRef<Path>,
    T: Display,
{
    let mut writer = BufWriter::new(File::create(path)?);
    write!(writer, "{contents}")?;
    writer.flush()
}

fn create_backup(path: &Path) -> io::Result<()> {
    const BACKUP_EXT: &str = "orig";
    let mut backup: PathBuf = OsString::from_iter([
        path.as_os_str(),
        ".".as_ref(),
        BACKUP_EXT.as_ref(),
    ])
    .into();
    let mut ext = String::new();
    let mut i = 0;
    loop {
        match File::options().write(true).create_new(true).open(&backup) {
            Ok(f) => {
                drop(f);
                std::fs::rename(path, &backup)?;
                return Ok(());
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                i += 1;
                ext.clear();
                write!(ext, "{BACKUP_EXT}{i}").unwrap();
                backup.set_extension(&ext);
            }
            Err(e) => return Err(e),
        }
    }
}

fn run() -> Result<(), ()> {
    let mut args = std::env::args_os();
    let arg0 = args.next();
    let bin = arg0
        .as_ref()
        .and_then(|s| Path::new(s).file_name()?.to_str())
        .unwrap_or("fix-ardour-lv2-index");
    let args = match args::parse(args) {
        Ok(Args::Run(args)) => args,
        Ok(Args::Help) => {
            print!("Usage: {bin} {USAGE}");
            return Ok(());
        }
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("See `{bin} --help`.");
            return Err(());
        }
    };
    let xml = match &args.input {
        args::Input::Stdin => std::io::read_to_string(io::stdin().lock())
            .map_err(|e| {
                eprintln!("error: could not read from stdin: {e}");
            })?,
        args::Input::Path(p) => std::fs::read_to_string(p).map_err(|e| {
            eprintln!("error: could not read session file: {e}");
        })?,
    };
    let patched = patch::patch(&xml).map_err(|e| {
        eprintln!("error: {e}");
    })?;
    let write_output = |path| {
        write_display(path, &patched).map_err(|e| {
            eprintln!("error: could not write output: {e}");
        })
    };
    match &args.output {
        args::Output::InPlace => {
            let args::Input::Path(path) = &args.input else {
                unreachable!();
            };
            create_backup(path).map_err(|e| {
                eprintln!("error: could not create backup: {e}");
            })?;
            write_output(path)?;
        }
        args::Output::Stdout => print!("{patched}"),
        args::Output::Path(p) => write_output(p)?,
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
