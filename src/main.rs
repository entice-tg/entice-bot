#![feature(conservative_impl_trait)]
#![recursion_limit = "1024"] // For error_chain

extern crate ctrlc;
extern crate erased_serde;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate clap;
extern crate config;
#[macro_use]
extern crate error_chain;
extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate telebot;
extern crate tokio_core;

use std::sync::Mutex;

use clap::{Arg, ArgMatches};

use std::path::Path;

mod settings;
mod errors;
mod commands;
mod stream;
mod entice;

use errors::*;
use settings::Settings;
use entice::EnticeBot;

fn is_file(path: String) -> ::std::result::Result<(), String> {
    if Path::new(&path).is_file() {
        Ok(())
    } else {
        Err("not a file/doesn't exist".to_owned())
    }
}

fn args<'a>() -> ArgMatches<'a> {
    app_from_crate!()
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .takes_value(true)
                .validator(is_file)
                .required(true),
        )
        .get_matches()
}

lazy_static! {
    static ref ENTICE : Mutex<EnticeBot> = Mutex::new(EnticeBot::new());
}

fn shutdown() {
    println!("shutting down event loop");
    ENTICE.lock().unwrap().stop().unwrap();
}

fn run() -> Result<()> {
    let matches = args();

    ctrlc::set_handler(shutdown).chain_err(|| "couldn't set ctrlc handler")?;

    Settings::add_file(matches.value_of("config").unwrap())?;

    let settings = Settings::try_fetch()?;

    let join_handle = { ENTICE.lock().unwrap().start(settings)? };

    join_handle.join().unwrap()?;
    println!("shut down complete");
    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }

        ::std::process::exit(1);
    }
}
