use win_dbg_logger::output_debug_string;
use std::sync::Mutex;

static mut DB: Option<Mutex<rusqlite::Connection>> = None;
const DEBUG: bool = true;

fn init_db() -> Result<rusqlite::Connection, Box<dyn std::error::Error + Send + Sync>> {
    let conn = if DEBUG {
        rusqlite::Connection::open("skyrim_search_se.db")?
    } else {
        rusqlite::Connection::open("")?
    };

    conn.execute_batch(r#"
        PRAGMA mmap_size=268435456;
        PRAGMA synchronous=OFF;
        PRAGMA journal_mode=OFF;
        DROP TABLE IF EXISTS forms;
        CREATE TABLE forms (
            id integer primary key not null,
            type integer,
            edid text unique
        );
        CREATE INDEX form_type ON forms (type);
        CREATE INDEX form_edid ON forms (edid);
        "#,
    )?;
    Ok(conn)
}

pub(crate) fn get_db() -> &'static Mutex<rusqlite::Connection> {
    unsafe {
        return if let Some(ref db) = DB {
            db
        } else {
            DB = match init_db() {
                Ok(db) => Some(Mutex::new(db)),
                Err(err) => {
                    output_debug_string(format!("failed to init_db: {}", err).as_str());
                    panic!(err);
                },
            };
            return DB.as_ref().unwrap();
        }
    }
}
