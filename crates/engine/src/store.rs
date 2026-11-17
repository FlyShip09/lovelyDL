//! SQLite storage on its own thread, so no async task ever blocks on disk.
//!
//! One table holds every job. The live queue is the rows that are not `cleared`; history is
//! the finished rows, searched through an FTS5 index over title, uploader, site and URL.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use rusqlite::{Connection, OptionalExtension, params};
use tokio::sync::oneshot;

use crate::model::{HistoryFilter, HistoryPage, Job};

/// One entry per schema version. `PRAGMA user_version` records how many have been applied.
const MIGRATIONS: &[&str] = &["
    CREATE TABLE jobs (
        id TEXT PRIMARY KEY,
        pos INTEGER NOT NULL DEFAULT 0,
        state TEXT NOT NULL,
        kind TEXT NOT NULL,
        cleared INTEGER NOT NULL DEFAULT 0,
        finished_at REAL,
        title TEXT NOT NULL DEFAULT '',
        uploader TEXT NOT NULL DEFAULT '',
        site TEXT NOT NULL DEFAULT '',
        url TEXT NOT NULL,
        data TEXT NOT NULL
    );
    CREATE INDEX jobs_finished ON jobs(finished_at DESC);
    CREATE VIRTUAL TABLE jobs_fts USING fts5(
        title, uploader, site, url,
        content='jobs', content_rowid='rowid', tokenize='unicode61 remove_diacritics 2'
    );
    CREATE TRIGGER jobs_ai AFTER INSERT ON jobs BEGIN
        INSERT INTO jobs_fts(rowid, title, uploader, site, url)
        VALUES (new.rowid, new.title, new.uploader, new.site, new.url);
    END;
    CREATE TRIGGER jobs_ad AFTER DELETE ON jobs BEGIN
        INSERT INTO jobs_fts(jobs_fts, rowid, title, uploader, site, url)
        VALUES ('delete', old.rowid, old.title, old.uploader, old.site, old.url);
    END;
    CREATE TRIGGER jobs_au AFTER UPDATE ON jobs BEGIN
        INSERT INTO jobs_fts(jobs_fts, rowid, title, uploader, site, url)
        VALUES ('delete', old.rowid, old.title, old.uploader, old.site, old.url);
        INSERT INTO jobs_fts(rowid, title, uploader, site, url)
        VALUES (new.rowid, new.title, new.uploader, new.site, new.url);
    END;
"];

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("could not start the storage thread: {0}")]
    Thread(#[from] std::io::Error),
    #[error("the database was written by a newer version of lovelyDL")]
    Newer,
}

type Op = Box<dyn FnOnce(&mut Connection) + Send>;

#[derive(Clone)]
pub struct Store {
    tx: mpsc::Sender<Op>,
}

fn migrate(conn: &mut Connection) -> Result<(), StoreError> {
    let version: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version as usize > MIGRATIONS.len() {
        return Err(StoreError::Newer);
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", index as u32 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

fn open(path: &Path) -> Result<Connection, StoreError> {
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// The value serde gives an enum variant, such as `"completed"`.
fn tag<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

/// Turns what the user typed into an FTS5 query: every word must match as a prefix.
fn fts_query(text: &str) -> Option<String> {
    let words: Vec<String> =
        text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(|w| format!("\"{w}\"*")).collect();
    (!words.is_empty()).then(|| words.join(" "))
}

fn read_job(data: String) -> Option<Job> {
    serde_json::from_str(&data).ok()
}

impl Store {
    /// Opens (or creates) the database and returns the live queue in order. A database that
    /// cannot be opened is moved aside and a new one started, so the app still launches.
    pub fn open(path: &Path) -> Result<(Self, Vec<Job>), StoreError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = match open(path) {
            Ok(conn) => conn,
            Err(StoreError::Newer) => return Err(StoreError::Newer),
            Err(error) => {
                tracing::error!(%error, "database is unusable; keeping it as .corrupt and starting fresh");
                let mut aside = PathBuf::from(path);
                aside.set_extension("corrupt");
                let _ = std::fs::rename(path, aside);
                open(path)?
            }
        };
        let jobs = {
            let mut stmt = conn.prepare("SELECT data FROM jobs WHERE cleared = 0 ORDER BY pos, rowid")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            rows.filter_map(Result::ok).filter_map(read_job).collect()
        };
        let (tx, rx) = mpsc::channel::<Op>();
        std::thread::Builder::new().name("lovelydl-store".into()).spawn(move || {
            let mut conn = conn;
            while let Ok(op) = rx.recv() {
                op(&mut conn);
            }
        })?;
        Ok((Self { tx }, jobs))
    }

    fn run(&self, op: impl FnOnce(&mut Connection) -> rusqlite::Result<()> + Send + 'static) {
        let sent = self.tx.send(Box::new(move |conn| {
            if let Err(error) = op(conn) {
                tracing::error!(%error, "database write failed");
            }
        }));
        if sent.is_err() {
            tracing::error!("storage thread is gone");
        }
    }

    async fn ask<R: Send + 'static>(
        &self,
        op: impl FnOnce(&mut Connection) -> rusqlite::Result<R> + Send + 'static,
    ) -> Option<R> {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(Box::new(move |conn| {
            let _ = tx.send(op(conn).map_err(|error| tracing::error!(%error, "database read failed")).ok());
        }));
        rx.await.ok().flatten()
    }

    pub fn save(&self, job: &Job, pos: usize) {
        let Ok(data) = serde_json::to_string(job) else { return };
        let job = job.clone();
        self.run(move |conn| {
            conn.execute(
                "INSERT INTO jobs (id, pos, state, kind, cleared, finished_at, title, uploader, site, url, data)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET pos = ?2, state = ?3, kind = ?4, cleared = ?5,
                     finished_at = ?6, title = ?7, uploader = ?8, site = ?9, url = ?10, data = ?11",
                params![
                    job.id,
                    pos as i64,
                    tag(&job.state),
                    tag(&job.kind),
                    job.cleared,
                    job.finished_at,
                    job.title,
                    job.uploader,
                    job.site,
                    job.url,
                    data
                ],
            )?;
            Ok(())
        });
    }

    /// Records the queue order: the position of each id is its index.
    pub fn save_order(&self, ids: Vec<String>) {
        self.run(move |conn| {
            let tx = conn.transaction()?;
            for (pos, id) in ids.iter().enumerate() {
                tx.execute("UPDATE jobs SET pos = ?1 WHERE id = ?2", params![pos as i64, id])?;
            }
            tx.commit()
        });
    }

    pub fn delete(&self, ids: Vec<String>) {
        self.run(move |conn| {
            let tx = conn.transaction()?;
            for id in &ids {
                tx.execute("DELETE FROM jobs WHERE id = ?1", [id])?;
            }
            tx.commit()
        });
    }

    pub async fn get(&self, id: String) -> Option<Job> {
        self.ask(move |conn| {
            conn.query_row("SELECT data FROM jobs WHERE id = ?1", [id], |r| r.get::<_, String>(0)).optional()
        })
        .await
        .flatten()
        .and_then(read_job)
    }

    /// Finished jobs, newest first. Marks rows whose file is no longer where it was saved.
    pub async fn history(&self, query: String, filter: HistoryFilter, offset: u32, limit: u32) -> HistoryPage {
        let page = self
            .ask(move |conn| {
                let fts = fts_query(&query);
                let mut clauses = vec![match filter {
                    HistoryFilter::All => "j.state IN ('completed', 'failed')",
                    HistoryFilter::Video => "j.state = 'completed' AND j.kind = 'video'",
                    HistoryFilter::Audio => "j.state = 'completed' AND j.kind = 'audio'",
                    HistoryFilter::Failed => "j.state = 'failed'",
                }];
                if fts.is_some() {
                    clauses.push("j.rowid IN (SELECT rowid FROM jobs_fts WHERE jobs_fts MATCH ?1)");
                }
                let filter = clauses.join(" AND ");
                // `?1` is unused without a search; binding an empty string keeps one code path.
                let fts = fts.unwrap_or_default();
                let total: u32 =
                    conn.query_row(&format!("SELECT count(*) FROM jobs j WHERE {filter} AND ?1 = ?1"), [&fts], |r| {
                        r.get(0)
                    })?;
                let mut stmt = conn.prepare(&format!(
                    "SELECT j.data FROM jobs j WHERE {filter} AND ?1 = ?1
                     ORDER BY j.finished_at DESC, j.rowid DESC LIMIT ?2 OFFSET ?3"
                ))?;
                let rows = stmt.query_map(params![fts, limit, offset], |r| r.get::<_, String>(0))?;
                let items = rows
                    .filter_map(Result::ok)
                    .filter_map(read_job)
                    .map(|mut job| {
                        job.file_missing = !job.output.is_empty() && !Path::new(&job.output).exists();
                        job
                    })
                    .collect();
                Ok(HistoryPage { items, total })
            })
            .await;
        page.unwrap_or(HistoryPage { items: Vec::new(), total: 0 })
    }

    /// Resolves once everything queued before it has been written.
    pub async fn flush(&self) {
        self.ask(|_| Ok(())).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DownloadRequest, JobState, MediaKind, Progress, Source};

    fn job(id: &str, title: &str, state: JobState, kind: MediaKind, finished: f64) -> Job {
        Job {
            id: id.into(),
            url: format!("https://www.youtube.com/watch?v={id}"),
            source: Source::Paste,
            title: title.into(),
            uploader: "Blender".into(),
            duration: Some(150.0),
            site: "youtube.com".into(),
            thumbnail: String::new(),
            kind,
            request: DownloadRequest::default(),
            state,
            progress: Progress::default(),
            planned: None,
            output: String::new(),
            file_missing: false,
            bytes: None,
            error: None,
            attempts: 0,
            group: None,
            cleared: false,
            created_at: 1.0,
            started_at: None,
            finished_at: (finished > 0.0).then_some(finished),
        }
    }

    #[tokio::test]
    async fn queue_survives_reopening_in_order() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let path = dir.path().join("db.sqlite3");
        {
            let (store, jobs) = Store::open(&path).unwrap_or_else(|e| panic!("{e}"));
            assert!(jobs.is_empty());
            store.save(&job("a", "First", JobState::Queued, MediaKind::Video, 0.0), 0);
            store.save(&job("b", "Second", JobState::Paused, MediaKind::Video, 0.0), 1);
            store.save(&job("c", "Third", JobState::Queued, MediaKind::Audio, 0.0), 2);
            store.save_order(vec!["c".into(), "a".into(), "b".into()]);
            let mut cleared = job("d", "Hidden", JobState::Completed, MediaKind::Video, 5.0);
            cleared.cleared = true;
            store.save(&cleared, 3);
            store.flush().await;
        }
        let (store, jobs) = Store::open(&path).unwrap_or_else(|e| panic!("{e}"));
        let ids: Vec<&str> = jobs.iter().map(|j| j.id.as_str()).collect();
        assert_eq!(ids, ["c", "a", "b"], "cleared jobs are not part of the live queue");
        assert_eq!(jobs[2].state, JobState::Paused);
        store.delete(vec!["a".into()]);
        assert!(store.get("a".into()).await.is_none());
        assert_eq!(store.get("d".into()).await.map(|j| j.title), Some("Hidden".into()));
    }

    #[tokio::test]
    async fn history_search_filters_and_missing_files() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let (store, _) = Store::open(&dir.path().join("db.sqlite3")).unwrap_or_else(|e| panic!("{e}"));
        let file = dir.path().join("kept.mp4");
        std::fs::write(&file, b"x").unwrap_or_else(|e| panic!("{e}"));
        let mut kept = job("v1", "Caminandes 3: Llamigos", JobState::Completed, MediaKind::Video, 30.0);
        kept.output = file.to_string_lossy().into_owned();
        let mut gone = job("a1", "Sintel soundtrack", JobState::Completed, MediaKind::Audio, 20.0);
        gone.output = dir.path().join("deleted.m4a").to_string_lossy().into_owned();
        store.save(&kept, 0);
        store.save(&gone, 1);
        store.save(&job("f1", "Élan vital", JobState::Failed, MediaKind::Video, 10.0), 2);
        store.save(&job("q1", "Still queued", JobState::Queued, MediaKind::Video, 0.0), 3);

        let all = store.history(String::new(), HistoryFilter::All, 0, 50).await;
        let ids: Vec<&str> = all.items.iter().map(|j| j.id.as_str()).collect();
        assert_eq!((ids, all.total), (vec!["v1", "a1", "f1"], 3), "newest first, queued excluded");
        assert_eq!((all.items[0].file_missing, all.items[1].file_missing), (false, true));

        let find = |q: &'static str, f| {
            let store = store.clone();
            async move {
                let page = store.history(q.into(), f, 0, 50).await;
                page.items.iter().map(|j| j.id.clone()).collect::<Vec<_>>()
            }
        };
        assert_eq!(find("llam", HistoryFilter::All).await, ["v1"], "prefix match on title");
        assert_eq!(find("blender sintel", HistoryFilter::All).await, ["a1"], "all words must match");
        assert_eq!(find("youtube.com", HistoryFilter::All).await.len(), 3, "site");
        assert_eq!(find("watch?v=f1", HistoryFilter::All).await, ["f1"], "url");
        assert_eq!(find("elan", HistoryFilter::All).await, ["f1"], "diacritics are folded");
        assert_eq!(find("\"); DROP TABLE jobs; --", HistoryFilter::All).await.len(), 0);
        assert_eq!(find("", HistoryFilter::Audio).await, ["a1"]);
        assert_eq!(find("", HistoryFilter::Video).await, ["v1"]);
        assert_eq!(find("", HistoryFilter::Failed).await, ["f1"]);

        // Paging, and edits reach the search index.
        let page = store.history(String::new(), HistoryFilter::All, 1, 1).await;
        assert_eq!((page.items[0].id.as_str(), page.total), ("a1", 3));
        let mut renamed = kept.clone();
        renamed.title = "Renamed".into();
        store.save(&renamed, 0);
        assert_eq!(find("llam", HistoryFilter::All).await.len(), 0);
        assert_eq!(find("renamed", HistoryFilter::All).await, ["v1"]);
    }

    #[test]
    fn migrations_are_idempotent_and_a_newer_database_is_refused() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let path = dir.path().join("db.sqlite3");
        for _ in 0..2 {
            let conn = open(&path).unwrap_or_else(|e| panic!("{e}"));
            let v: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap_or(0);
            assert_eq!(v as usize, MIGRATIONS.len());
        }
        let conn = Connection::open(&path).unwrap_or_else(|e| panic!("{e}"));
        conn.pragma_update(None, "user_version", 99).unwrap_or_else(|e| panic!("{e}"));
        drop(conn);
        assert!(matches!(Store::open(&path), Err(StoreError::Newer)));
    }

    #[test]
    fn a_corrupt_database_is_set_aside() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let path = dir.path().join("db.sqlite3");
        std::fs::write(&path, b"this is not a sqlite file, not even close").unwrap_or_else(|e| panic!("{e}"));
        let (_, jobs) = Store::open(&path).unwrap_or_else(|e| panic!("{e}"));
        assert!(jobs.is_empty());
        assert!(path.with_extension("corrupt").exists());
    }
}
