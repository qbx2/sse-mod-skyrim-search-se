use anyhow::{anyhow, Context};
use winapi::ctypes::{c_void, c_char};
use std::ffi::{CStr, CString};
use std::intrinsics::transmute;
use detour::static_detour;
use clap::{SubCommand, Arg, AppSettings};
use win_dbg_logger::output_debug_string;
use crate::db;
use rusqlite::{NO_PARAMS, Statement};
use std::option::NoneError;
use rusqlite::types::ValueRef;
use late_static::LateStatic;
use crate::log::Loggable;
use rusqlite::params;
use std::sync::mpsc::Sender;
use crate::db::Job;
use std::sync::{Arc, Mutex, Condvar};

static_detour! {
    static ProcessConsoleInput: fn(usize, i64, i64, i64);
}

const SKYRIM_SEARCH_COMMANDS: [&str; 4] = ["ss", "sss", "skyrimsearch", "skyrimsearchse"];

fn get_clap<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new("skyrim-search-se")
        .version("0.1")
        .setting(AppSettings::DisableHelpSubcommand)
        .setting(AppSettings::InferSubcommands)
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(Arg::with_name("debug")
                .long("debug")
                .global(true))
        .subcommand(SubCommand::with_name("query")
            .about("execute raw query. quote your query as in unix shell if needed.")
            .setting(AppSettings::TrailingVarArg)
            .arg(Arg::with_name("sql")
                .help("SQLite SQL")
                .required(true)
                .multiple(true))
            .arg(Arg::with_name("int-as-decimal")
                .long("int-as-decimal")
                .help("print integer in decimal format. \
                          otherwise, it's printed in hexademical format.")))
        .subcommand(SubCommand::with_name("npc")
            .about("search npc and its reference")
            .arg(Arg::with_name("query")
                .help("search query (e.g. name, edid, form_id, ref_id)")
                .required(true)
                .multiple(true)))
}

fn new_process_console_input(param1: usize, param2: i64, param3: i64, param4: i64) {
    let mut print_usage = false;
    let result: anyhow::Result<bool> = (|| {
        let input = match unsafe {
            CStr::from_ptr(*((param1 + 0x38) as *const *const c_char)).to_str()
        } {
            Ok(input) => input,
            Err(err) => {
                output_debug_string(err.to_string().as_str());
                return Ok(false);
            }
        };
        if input.len() == 0 {
            return Ok(false);
        }
        let input = match shlex::split(input) {
            Some(result) => result,
            None => {
                if let Some(command) = input.trim_start().split_ascii_whitespace().next() {
                    if SKYRIM_SEARCH_COMMANDS.contains(&command) {
                        print("skyrim-search-se: parse failed; falling back to skyrim engine");
                    }
                }
                return Ok(false);
            }
        };
        let command = input[0].to_ascii_lowercase();
        if !SKYRIM_SEARCH_COMMANDS.contains(&command.as_str()) {
            print_usage = command == "help";
            return Ok(false);
        }

        let matches = get_clap().get_matches_from_safe(input)?;

        if matches.is_present("debug") {
            print(format!("ArgMatches: {:?}", matches));
        }

        static CREATE_INDEX: std::sync::Once = std::sync::Once::new();
        CREATE_INDEX.call_once(|| {
            let pair = Arc::new((Mutex::new(()), Condvar::new()));
            let pair2 = Arc::clone(&pair);
            S.task_queue.send(Box::new(move |db| {
                db.execute_batch(r#"
                     CREATE INDEX npc_editor_id ON npc (editor_id);
                     CREATE INDEX npc_name ON npc (name);
                     CREATE INDEX actor_base_form_id ON actor (base_form_id);
                 "#).logging_ok();

                pair2.1.notify_one();

                Ok(())
            })).map_err(|e| anyhow!(e.to_string())).logging_ok();
            let (lock, cond) = &*pair;
            if let Ok(guard) = lock.lock() {
                cond.wait(guard).map_err(|e| anyhow!(e.to_string())).logging_ok();
            };
        });

        if let Some(matches) = matches.subcommand_matches("query") {
            process_query_command(matches)?;
        } else if let Some(matches) = matches.subcommand_matches("npc") {
            process_npc_command(matches)?;
        }
        Ok(true)
    })();
    if let Err(ref err) = result {
        print(format!("{:#}", err));
    }
    if let Ok(false) = result {
        ProcessConsoleInput.call(param1, param2, param3, param4);
    }
    if print_usage {
        print("skyrim-search-se usage: ss --help");
    }
}

fn process_query_command(matches: &clap::ArgMatches) -> anyhow::Result<()> {
    let sql = matches.values_of("sql").unwrap().collect::<Vec<&str>>().join(" ");
    let db = db::DB.lock().unwrap();
    let mut stmt: Statement = db.prepare(sql.as_str()).context("prepare error")?;

    if matches.is_present("debug") {
        print(format!("stmt: {:?}", stmt));
    }

    let rows = stmt.query(NO_PARAMS).context("query error")?;
    print_rows(rows, matches)
}

fn process_npc_command(matches: &clap::ArgMatches) -> anyhow::Result<()> {
    let db = db::DB.lock().unwrap();
    let query: String = matches.values_of("query").unwrap().collect::<Vec<&str>>().join(" ");

    let mut stmt;
    let rows;

    if let Ok(id) = i64::from_str_radix(query.trim_start_matches("0x"), 16) {
        stmt = db.prepare_cached(
            "SELECT npc.*, actor.form_id as ref_id FROM npc \
             LEFT JOIN actor ON npc.form_id = actor.base_form_id \
             WHERE npc.editor_id LIKE ?1 OR npc.name LIKE ?1 \
             OR npc.form_id=?2 OR actor.form_id=?2"
        ).context("prepare error")?;

        if matches.is_present("debug") {
            print(format!("stmt: {:?}", *stmt));
        }

        rows = stmt.query(params![format!("%{}%", query), id])
            .context("query error")?;
    } else {
        stmt = db.prepare_cached(
            "SELECT npc.*, actor.form_id as ref_id FROM npc \
             LEFT JOIN actor ON npc.form_id = actor.base_form_id \
             WHERE npc.editor_id LIKE ?1 OR npc.name LIKE ?1"
        ).context("prepare error")?;

        if matches.is_present("debug") {
            print(format!("stmt: {:?}", *stmt));
        }

        rows = stmt.query(params![format!("%{}%", query)])
            .context("query error")?;
    }

    print_rows(rows, matches)
}

fn print_rows(mut rows: rusqlite::Rows, matches: &clap::ArgMatches) -> anyhow::Result<()> {
    let print_int_as_decimal = matches.is_present("int-as-decimal");
    let column_count = match rows.column_count() {
        Some(count) => count,
        None => anyhow::bail!("no data"),
    };

    let mut ptable = prettytable::Table::new();
    let _: Result<(), NoneError> = try {
        let names = rows.column_names()?;
        ptable.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        ptable.set_titles(
            names
                .into_iter()
                .map(prettytable::Cell::new)
                .collect()
        );
    };
    loop {
        let row = match rows.next().map_err(anyhow::Error::new) {
            Ok(Some(row)) => row,
            Ok(None) => break,
            Err(err) => anyhow::bail!(err.context("rows.next() error")),
        };
        let mut cells = Vec::with_capacity(column_count);
        for i in 0..column_count {
            let column = row.get_raw(i);
            let repr = match column {
                ValueRef::Null => String::from("<null>"),
                ValueRef::Integer(v) => {
                    if print_int_as_decimal {
                        v.to_string()
                    } else {
                        format!("{:#x}", v)
                    }
                },
                ValueRef::Real(v) => v.to_string(),
                ValueRef::Text(v) => String::from_utf8_lossy(v).to_string(),
                ValueRef::Blob(v) => format!("<{}-byte blob>", v.len()),
            };
            cells.push(prettytable::Cell::new(repr.as_str()));
        }
        ptable.add_row(prettytable::Row::new(cells));
    }
    print(ptable.to_string());
    Ok(())
}

struct State {
    console_context: *const *const c_void,
    print_to_console: extern "C" fn(*const c_void, *const c_char, ...) -> (),
    task_queue: Sender<Job>,
}
unsafe impl Sync for State {}
static S: LateStatic<State> = LateStatic::new();

pub(crate) fn print<T: Into<Vec<u8>>>(msg: T) {
    let msg = msg.into();
    let msg = String::from_utf8_lossy(msg.as_ref());
    let msgs = msg.split("\n");
    // The print_to_console's internal buffer size is 1024.
    // ensure each lines not to overflow
    let chunks = msgs.flat_map(|msg| msg.as_bytes().chunks(1024));
    let chunks: Vec<Result<CString, _>> = chunks.map(CString::new).collect();

    let result: anyhow::Result<()> = try {
        unsafe {
            let console_context = S.console_context;
            if *console_context != std::ptr::null() {
                for msg in chunks {
                    (S.print_to_console)(
                        *console_context,
                        "%s\0".as_ptr() as *const c_char,
                        msg?.as_c_str().as_ptr(),
                    );
                }
            }
        }
    };

    result.logging_ok();
}

pub(crate) unsafe fn init(image_base: usize) -> anyhow::Result<()> {
    LateStatic::assign(&S, State {
        console_context: transmute(image_base + 0x2f000f0),
        print_to_console: transmute(image_base + 0x85c290),
        task_queue: db::TASK_QUEUE.lock().unwrap().clone(),
    });

    let target_addr = transmute(image_base + 0x2e75f0);
    ProcessConsoleInput.initialize(target_addr, new_process_console_input)
        .context("initialize")?;
    ProcessConsoleInput.enable().context("enable")?;

    Ok(())
}
