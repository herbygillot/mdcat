// Copyright 2018 Sebastian Wiesner <sebastian@swsnr.de>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

// 	http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![deny(warnings)]
#![deny(missing_docs)]
#![cfg_attr(feature = "cargo-clippy", deny(clippy))]

//! Show CommonMark documents on TTYs.

extern crate base64;
#[macro_use]
extern crate clap;
extern crate pulldown_cmark;
extern crate syntect;
extern crate termion;

use std::path::PathBuf;
use std::io::prelude::*;
use std::io::stdin;
use std::fs::File;
use std::error::Error;
use std::str::FromStr;
use pulldown_cmark::Parser;
use syntect::parsing::SyntaxSet;

mod terminal;
mod highlighting;
mod commonmark;

/// Colour options, for the --colour option.
#[derive(Debug)]
enum Colour {
    Yes,
    No,
    Auto,
}

#[derive(Debug)]
struct InvalidColour {}

impl ToString for InvalidColour {
    fn to_string(&self) -> String {
        String::from("invalid colour setting")
    }
}

impl FromStr for Colour {
    type Err = InvalidColour;

    fn from_str(value: &str) -> Result<Self, InvalidColour> {
        match value.to_lowercase().as_str() {
            "yes" => Ok(Colour::Yes),
            "no" => Ok(Colour::No),
            "auto" => Ok(Colour::Auto),
            _ => Err(InvalidColour {}),
        }
    }
}

/// Read input for `filename`.
///
/// If `filename` is `-` read from standard input, otherwise try to open and
/// read the given file.
fn read_input<T: AsRef<str>>(filename: T) -> std::io::Result<(PathBuf, String)> {
    let cd = std::env::current_dir()?;
    let mut buffer = String::new();

    if filename.as_ref() == "-" {
        stdin().read_to_string(&mut buffer)?;
        Ok((cd, buffer))
    } else {
        let mut source = File::open(filename.as_ref())?;
        source.read_to_string(&mut buffer)?;
        let base_dir = cd.join(filename.as_ref())
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(cd);
        Ok((base_dir, buffer))
    }
}

fn process_arguments(args: Arguments) -> Result<(), Box<Error>> {
    let (base_dir, input) = read_input(args.filename)?;
    let parser = Parser::new(&input);

    if args.dump_events {
        commonmark::dump_events(&mut std::io::stdout(), parser)?;
        Ok(())
    } else {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        commonmark::push_tty(
            &mut std::io::stdout(),
            args.columns,
            parser,
            &base_dir,
            args.format,
            syntax_set,
        )?;
        Ok(())
    }
}

/// Represent command line arguments.
#[derive(Debug)]
struct Arguments {
    filename: String,
    format: terminal::Format,
    columns: u16,
    dump_events: bool,
}

impl Arguments {
    /// Create command line arguments from matches.
    fn from_matches(matches: &clap::ArgMatches) -> clap::Result<Self> {
        let format = match value_t!(matches, "colour", Colour)? {
            Colour::No => terminal::Format::NoColours,
            Colour::Yes => terminal::Format::auto_detect(true),
            Colour::Auto => terminal::Format::auto_detect(false),
        };
        let filename = value_t!(matches, "filename", String)?;
        let dump_events = matches.is_present("dump_events");
        let columns = value_t!(matches, "columns", u16)?;

        Ok(Arguments {
            filename,
            columns,
            dump_events,
            format,
        })
    }
}

fn main() {
    use clap::*;
    let columns = terminal::columns().to_string();
    let app = app_from_crate!()
        .setting(AppSettings::UnifiedHelpMessage)
        .arg(
            Arg::with_name("dump_events")
                .long("dump-events")
                .help("Dump Markdown parser events and exit"),
        )
        .arg(
            Arg::with_name("columns")
                .long("columns")
                .help("Maximum number of columns to use for output")
                .default_value(&columns),
        )
        .arg(
            Arg::with_name("colour")
                .short("c")
                .long("colour")
                .help("Whether to enable colours.")
                .possible_values(&["yes", "no", "auto"])
                .default_value("auto"),
        )
        .arg(
            Arg::with_name("filename")
                .help("The file to read.  If - read from standard input instead")
                .default_value("-"),
        );

    let matches = app.get_matches();
    let arguments = Arguments::from_matches(&matches).unwrap_or_else(|e| e.exit());
    match process_arguments(arguments) {
        Ok(_) => std::process::exit(0),
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}
