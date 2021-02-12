use win_dbg_logger::output_debug_string;
use std::sync::Mutex;
use anyhow::Context;
use lazy_static::lazy_static;
use crate::log::Loggable;

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

    pub static ref TASK_QUEUE: Mutex<std::sync::mpsc::Sender<Job>> = {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(|| Worker(rx).worker());
        Mutex::new(tx)
    };
}

pub(crate) type Job = Box<dyn FnOnce(&rusqlite::Connection) -> anyhow::Result<()> + Send + 'static>;

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
            editor_id text collate nocase,
            name text collate nocase
        );

        DROP TABLE IF EXISTS actor;
        CREATE TABLE actor (
            form_id integer primary key not null,
            base_form_id integer
        );

        DROP TABLE IF EXISTS cell;
        CREATE TABLE cell (
            form_id integer primary key not null,
            editor_id text collate nocase,
            name text collate nocase
        );
        "#,
    ).context("init_schema error")?;

    Ok(conn)
}

struct Worker(std::sync::mpsc::Receiver<Job>);

impl Worker {
    fn worker(self) {
        let task_queue = self.0;
        loop {
            let job = task_queue.recv().unwrap();
            let db = DB.lock().unwrap();
            let mut num_jobs = 1;
            Self::process_job(&db, job).logging_ok();
            for job in task_queue.try_iter() {
                num_jobs += 1;
                Self::process_job(&db, job).logging_ok();
            }
            output_debug_string(format!("processed {} jobs", num_jobs).as_str());
        }
    }

    fn process_job(db: &rusqlite::Connection, msg: Job) -> anyhow::Result<()> {
        msg(db)
    }
}

pub(crate) fn init_index(db: &rusqlite::Connection) -> rusqlite::Result<()> {
    db.execute_batch(r#"
        CREATE INDEX IF NOT EXISTS npc_editor_id ON npc (editor_id);
        CREATE INDEX IF NOT EXISTS npc_name ON npc (name);

        CREATE INDEX IF NOT EXISTS actor_base_form_id ON actor (base_form_id);

        CREATE INDEX IF NOT EXISTS cell_editor_id ON cell (editor_id);
        CREATE INDEX IF NOT EXISTS cell_name ON cell (name);
     "#)
}