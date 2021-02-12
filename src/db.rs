use win_dbg_logger::output_debug_string;
use std::sync::Mutex;
use anyhow::Context;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref DB: Mutex<rusqlite::Connection> = {
        match init_db().context("init_db error") {
            Ok(db) => Mutex::new(db),
            Err(err) => {
                output_debug_string(format!("{:#}", err).as_str());
                panic!(format!("{:#}", err));
            }
        }
    };
}

fn init_db() -> anyhow::Result<rusqlite::Connection> {
    let conn = if crate::DEBUG {
        rusqlite::Connection::open("skyrim_search_se.db").context("open error")?
    } else {
        rusqlite::Connection::open("").context("open error")?
    };

    conn.execute_batch(r#"
        PRAGMA mmap_size=268435456;
        PRAGMA synchronous=OFF;
        PRAGMA journal_mode=OFF;

        DROP TABLE IF EXISTS npc;
        CREATE TABLE npc (
            form_id integer primary key not null,
            editor_id text unique collate nocase,
            name text collate nocase
        );

        DROP TABLE IF EXISTS actor;
        CREATE TABLE actor (
            form_id integer primary key not null,
            base_form_id integer
        );
        "#,
    ).context("init_schema error")?;
    Ok(conn)
}
