use crate::db::Job;
use crate::form::qust::TESQuest;
use crate::form::TESForm;
use crate::log::Loggable;
use crate::{console, db};
use anyhow::{anyhow, Context};
use clap::{AppSettings, Arg, SubCommand};
use late_static::LateStatic;
use rusqlite::params;
use rusqlite::types::ValueRef;
use rusqlite::{Statement, NO_PARAMS};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};

pub(crate) enum ProcessResult {
    Processed,
    Fallback,
    FallbackAndPrintUsage,
}

pub const SKYRIM_SEARCH_COMMANDS: [&str; 4] = ["ss", "sss", "skyrimsearch", "skyrimsearchse"];

pub fn get_clap<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new("skyrim-search-se")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Author: qbx2/lukasaldersley | GitHub: https://github.com/qbx2/sse-mod-skyrim-search-se")
        .setting(AppSettings::DisableHelpSubcommand)
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(Arg::with_name("debug").long("debug").global(true))
        .subcommand(
            SubCommand::with_name("raw")
                .about("execute raw query. quote your query as in unix shell if needed.")
                .setting(AppSettings::TrailingVarArg)
                .arg(
                    Arg::with_name("sql")
                        .help("SQLite SQL")
                        .required(true)
                        .multiple(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("npc")
                .alias("npcs")
                .about("search npc/reference")
                .arg(
                    Arg::with_name("query")
                        .help("search query (e.g. name, edid, form_id, ref_id)")
                        .required(true)
                        .multiple(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("cell")
                .alias("cells")
                .about("search cell (location)")
                .arg(
                    Arg::with_name("query")
                        .help("search query (e.g. name, edid, form_id)")
                        .required(true)
                        .multiple(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("quest")
                .alias("quests")
                .about("search quest")
                .arg(
                    Arg::with_name("query")
                        .help("search query (e.g. name, edid, form_id)")
                        .required(true)
                        .multiple(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("quest_stage")
                .alias("quest_stages")
                .alias("qs")
                .alias("queststage")
                .alias("queststages")
                .about("search quest (prints additional stage information)")
                .arg(
                    Arg::with_name("query")
                        .help("search query (e.g. name, edid, form_id)")
                        .required(true)
                        .multiple(true),
                ),
        )
}

struct State {
    task_queue: Sender<Job>,
}
unsafe impl Sync for State {}
static S: LateStatic<State> = LateStatic::new();

pub(crate) fn process_console_input(input: &str) -> anyhow::Result<ProcessResult> {
    if input.is_empty() {
        return Ok(ProcessResult::Fallback);
    }
    let input = match shlex::split(input) {
        Some(result) => result,
        None => {
            if let Some(command) = input.trim_start().split_ascii_whitespace().next() {
                if SKYRIM_SEARCH_COMMANDS.contains(&command) {
                    console::print("skyrim-search-se: parse failed; falling back to skyrim engine");
                }
            }
            return Ok(ProcessResult::Fallback);
        }
    };
    let command = input[0].to_ascii_lowercase();
    if !SKYRIM_SEARCH_COMMANDS.contains(&command.as_str()) {
        return if command == "help" {
            Ok(ProcessResult::FallbackAndPrintUsage)
        } else {
            Ok(ProcessResult::Fallback)
        };
    }

    let matches = get_clap().get_matches_from_safe(input)?;

    if matches.is_present("debug") {
        console::print(format!("ArgMatches: {:?}", matches));
    }

    static CREATE_INDEX: std::sync::Once = std::sync::Once::new();
    CREATE_INDEX.call_once(|| {
        let pair = Arc::new((Mutex::new(()), Condvar::new()));
        let pair2 = Arc::clone(&pair);
        S.task_queue
            .send(Box::new(move |db| {
                db::init_index(db).logging_ok();

                pair2.1.notify_one();

                Ok(())
            }))
            .map_err(|e| anyhow!(e.to_string()))
            .logging_ok();
        let (lock, cond) = &*pair;
        if let Ok(guard) = lock.lock() {
            cond.wait(guard)
                .map_err(|e| anyhow!(e.to_string()))
                .logging_ok();
        };
    });

    if let Some(matches) = matches.subcommand_matches("raw") {
        process_raw_query_command(matches)?;
    } else if let Some(matches) = matches.subcommand_matches("npc") {
        process_npc_command(matches)?;
    } else if let Some(matches) = matches.subcommand_matches("cell") {
        process_cell_command(matches)?;
    } else if let Some(matches) = matches.subcommand_matches("quest") {
        process_quest_command(matches)?;
    } else if let Some(matches) = matches.subcommand_matches("quest_stage") {
        process_quest_log_command(matches)?;
    }
    Ok(ProcessResult::Processed)
}

pub fn process_raw_query_command(matches: &clap::ArgMatches) -> anyhow::Result<()> {
    let sql = matches
        .values_of("sql")
        .unwrap()
        .collect::<Vec<&str>>()
        .join(" ");
    let db = db::DB.lock().unwrap();
    let mut stmt: Statement = db.prepare(sql.as_str()).context("prepare error")?;

    if matches.is_present("debug") {
        console::print(format!("stmt: {:?}", stmt));
    }

    let rows = stmt.query(NO_PARAMS).context("query error")?;
    print_rows(rows, convert_row)?;

    Ok(())
}

pub fn process_npc_command(matches: &clap::ArgMatches) -> anyhow::Result<()> {
    let db = db::DB.lock().unwrap();
    let query: String = matches
        .values_of("query")
        .unwrap()
        .collect::<Vec<&str>>()
        .join(" ");

    let mut stmt;
    let rows;

    if let Ok(id) = i64::from_str_radix(query.trim_start_matches("0x"), 16) {
        stmt = db
            .prepare_cached(
                "SELECT npc.*, actor.form_id as ref_id FROM npc \
             LEFT JOIN actor ON npc.form_id = actor.base_form_id \
             WHERE npc.editor_id LIKE ?1 OR npc.name LIKE ?1 \
             OR npc.form_id=?2 OR actor.form_id=?2",
            )
            .context("prepare error")?;

        if matches.is_present("debug") {
            console::print(format!("stmt: {:?}", *stmt));
        }

        rows = stmt
            .query(params![format!("%{}%", query), id])
            .context("query error")?;
    } else {
        stmt = db
            .prepare_cached(
                "SELECT npc.*, actor.form_id as ref_id FROM npc \
             LEFT JOIN actor ON npc.form_id = actor.base_form_id \
             WHERE npc.editor_id LIKE ?1 OR npc.name LIKE ?1",
            )
            .context("prepare error")?;

        if matches.is_present("debug") {
            console::print(format!("stmt: {:?}", *stmt));
        }

        rows = stmt
            .query(params![format!("%{}%", query)])
            .context("query error")?;
    }

    print_rows(rows, convert_row)?;

    Ok(())
}

pub fn process_cell_command(matches: &clap::ArgMatches) -> anyhow::Result<()> {
    let db = db::DB.lock().unwrap();
    let query: String = matches
        .values_of("query")
        .unwrap()
        .collect::<Vec<&str>>()
        .join(" ");

    let mut stmt;
    let rows;

    if let Ok(id) = i64::from_str_radix(query.trim_start_matches("0x"), 16) {
        stmt = db
            .prepare_cached(
                "SELECT * FROM cell WHERE editor_id LIKE ?1 OR name LIKE ?1 OR form_id=?2",
            )
            .context("prepare error")?;

        if matches.is_present("debug") {
            console::print(format!("stmt: {:?}", *stmt));
        }

        rows = stmt
            .query(params![format!("%{}%", query), id])
            .context("query error")?;
    } else {
        stmt = db
            .prepare_cached("SELECT * FROM cell WHERE editor_id LIKE ?1 OR name LIKE ?1")
            .context("prepare error")?;

        if matches.is_present("debug") {
            console::print(format!("stmt: {:?}", *stmt));
        }

        rows = stmt
            .query(params![format!("%{}%", query)])
            .context("query error")?;
    }

    print_rows(rows, convert_row)?;

    Ok(())
}

pub fn process_quest_command(matches: &clap::ArgMatches) -> anyhow::Result<()> {
    let db = db::DB.lock().unwrap();
    let query: String = matches
        .values_of("query")
        .unwrap()
        .collect::<Vec<&str>>()
        .join(" ");

    let mut stmt;
    let rows;

    if let Ok(id) = i64::from_str_radix(query.trim_start_matches("0x"), 16) {
        stmt = db
            .prepare_cached(
                "SELECT * FROM quest WHERE editor_id LIKE ?1 OR name LIKE ?1 OR form_id=?2",
            )
            .context("prepare error")?;

        if matches.is_present("debug") {
            console::print(format!("stmt: {:?}", *stmt));
        }

        rows = stmt
            .query(params![format!("%{}%", query), id])
            .context("query error")?;
    } else {
        stmt = db
            .prepare_cached("SELECT * FROM quest WHERE editor_id LIKE ?1 OR name LIKE ?1")
            .context("prepare error")?;

        if matches.is_present("debug") {
            console::print(format!("stmt: {:?}", *stmt));
        }

        rows = stmt
            .query(params![format!("%{}%", query)])
            .context("query error")?;
    }

    print_rows(rows, convert_row)?;

    Ok(())
}

pub fn process_quest_log_command(matches: &clap::ArgMatches) -> anyhow::Result<()> {
    let db = db::DB.lock().unwrap();
    let query: String = matches
        .values_of("query")
        .unwrap()
        .collect::<Vec<&str>>()
        .join(" ");

    let mut stmt;
    let rows;

    if let Ok(id) = i64::from_str_radix(query.trim_start_matches("0x"), 16) {
        stmt = db.prepare_cached(
            "SELECT quest.*, stage, log FROM quest LEFT JOIN quest_stage ON quest.form_id = quest_stage.form_id \
             WHERE log IS NOT NULL AND (quest.editor_id LIKE ?1 OR quest.name LIKE ?1 OR quest.form_id=?2)"
        ).context("prepare error")?;

        if matches.is_present("debug") {
            console::print(format!("stmt: {:?}", *stmt));
        }

        rows = stmt
            .query(params![format!("%{}%", query), id])
            .context("query error")?;
    } else {
        stmt = db
            .prepare_cached(
                "SELECT quest.*, stage, log FROM quest LEFT JOIN quest_stage \
             ON quest.form_id = quest_stage.form_id \
             WHERE log IS NOT NULL AND (quest.editor_id LIKE ?1 OR quest.name LIKE ?1)",
            )
            .context("prepare error")?;

        if matches.is_present("debug") {
            console::print(format!("stmt: {:?}", *stmt));
        }

        rows = stmt
            .query(params![format!("%{}%", query)])
            .context("query error")?;
    }

    let rows: rusqlite::Rows = rows;

    let num_rows = print_rows(rows, |row: &rusqlite::Row| {
        let column_count = row.column_count();
        let mut cells = Vec::with_capacity(column_count);
        for i in 0..column_count - 1 {
            let column = row.get_raw(i);
            let repr = repr_column(row.column_name(i).ok(), column);
            cells.push(prettytable::Cell::new(repr.as_str()));
        }
        let description: anyhow::Result<std::borrow::Cow<str>> = (|| {
            let form_id = row.get_raw("form_id").as_i64()? as u32;
            let stage = row.get_raw("stage").as_i64()?;
            let quest: &TESQuest =
                unsafe { &*(TESForm::look_up_by_id(form_id) as *const TESQuest) };
            let index = quest
                .get_log(stage as u16)
                .ok_or_else(|| anyhow!("invalid data"))?;
            let log_entry = if let Some(log_entry) = index.head.into_iter().next() {
                quest.get_log_description(unsafe { &*log_entry })
            } else {
                std::borrow::Cow::from("")
            };
            Ok(log_entry)
        })();

        cells.push(prettytable::Cell::new(
            description
                .unwrap_or_else(|e| e.to_string().into())
                .as_ref(),
        ));
        prettytable::Row::new(cells)
    })?;

    if num_rows == 0 {
        console::print("Change your query or try loading a save?");
    }

    Ok(())
}

fn print_rows<F>(mut rows: rusqlite::Rows, f: F) -> anyhow::Result<usize>
where
    F: Fn(&rusqlite::Row) -> prettytable::Row,
{
    let mut num_rows = 0;

    if rows.column_count().is_none() {
        anyhow::bail!("no data");
    }

    let mut ptable = prettytable::Table::new();
    set_titles(&mut rows, &mut ptable);
    loop {
        let row = match rows.next().map_err(anyhow::Error::new) {
            Ok(Some(row)) => row,
            Ok(None) => break,
            Err(err) => anyhow::bail!(err.context("rows.next() error")),
        };
        ptable.add_row(f(row));
        num_rows += 1;
    }

    if num_rows > 0 {
        console::print(ptable.to_string());
    } else {
        console::print("No result");
    }

    Ok(num_rows)
}

fn convert_row(row: &rusqlite::Row) -> prettytable::Row {
    let column_count = row.column_count();
    let mut cells = Vec::with_capacity(column_count);
    for i in 0..column_count {
        let column = row.get_raw(i);
        let repr = repr_column(row.column_name(i).ok(), column);
        cells.push(prettytable::Cell::new(repr.as_str()));
    }
    prettytable::Row::new(cells)
}

fn repr_column(name: Option<&str>, column: ValueRef) -> String {
    match column {
        ValueRef::Null => String::from("<null>"),
        ValueRef::Integer(v) => match name {
            Some(name) if name.contains("id") => format!("{:08X}", v),
            _ => v.to_string(),
        },
        ValueRef::Real(v) => v.to_string(),
        ValueRef::Text(v) => String::from_utf8_lossy(v).to_string(),
        ValueRef::Blob(v) => format!("<{}-byte blob>", v.len()),
    }
}

fn set_titles(rows: &mut rusqlite::Rows, table: &mut prettytable::Table) -> Option<()> {
    let names = rows.column_names()?;
    console::print(fmt!("names: {:?}", names));
    table.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(names.into_iter().map(prettytable::Cell::new).collect());
    Some(())
}

pub(crate) unsafe fn init(_image_base: usize) -> anyhow::Result<()> {
    LateStatic::assign(
        &S,
        State {
            task_queue: db::TASK_QUEUE.lock().unwrap().clone(),
        },
    );

    Ok(())
}
