//! `<id>.md` files in one directory are the only state. A process-wide lock
//! serializes read-modify-write; each file replacement is atomic.
use crate::{
    Error,
    todo::{self, Todo},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

pub struct Store {
    dir: PathBuf,
    lock: Mutex<()>,
}

fn now() -> String {
    let format = time::macros::format_description!(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z"
    );
    time::OffsetDateTime::now_utc()
        .format(format)
        .expect("UTC timestamps format")
}

impl Store {
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, Error> {
        let dir = dir.as_ref();
        if dir.as_os_str().to_string_lossy().trim().is_empty() {
            return Err(Error::Invalid("APP_DATA_DIR must not be blank".into()));
        }
        fs::create_dir_all(dir)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            lock: Mutex::new(()),
        })
    }

    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn guard(&self) -> MutexGuard<'_, ()> {
        // A panic mid-request leaves no partial file behind, so the lock stays usable.
        self.lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn path(&self, id: &str) -> Result<PathBuf, Error> {
        todo::check_id(id)?;
        Ok(self.dir.join(format!("{id}.md")))
    }

    /// `(id, markdown)` for every `<id>.md` file; other names are ignored.
    fn files(&self) -> Result<Vec<(String, String)>, Error> {
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let name = entry?.file_name();
            let Some(id) = name.to_str().and_then(|n| n.strip_suffix(".md")) else {
                continue;
            };
            if todo::check_id(id).is_ok() {
                let text = fs::read_to_string(self.dir.join(&name))?;
                files.push((id.to_owned(), text));
            }
        }
        Ok(files)
    }

    fn read(&self, id: &str) -> Result<Todo, Error> {
        match fs::read_to_string(self.path(id)?) {
            Ok(text) => todo::parse_document(id, &text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(Error::NotFound(format!("todo not found: {id}")))
            }
            Err(e) => Err(e.into()),
        }
    }

    fn write(&self, todo: &Todo) -> Result<(), Error> {
        let path = self.path(&todo.id)?;
        let temporary = self.dir.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| -> Result<(), Error> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(todo::render_document(todo)?.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temporary, &path)?;
            File::open(&self.dir)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    /// `done: None` lists everything, open items first.
    pub fn list(&self, done: Option<bool>) -> Result<Vec<Todo>, Error> {
        let _guard = self.guard();
        let mut todos = self
            .files()?
            .into_iter()
            .map(|(id, text)| todo::parse_document(&id, &text))
            .collect::<Result<Vec<_>, _>>()?;
        todos.retain(|t| done.is_none_or(|done| t.meta.done_at.is_some() == done));
        todo::sort(&mut todos);
        Ok(todos)
    }

    pub fn get(&self, id: &str) -> Result<Todo, Error> {
        let _guard = self.guard();
        self.read(id)
    }

    pub fn create(&self, fields: &Value) -> Result<Todo, Error> {
        if fields.get("title").is_none() {
            return Err(Error::Invalid("title is required".into()));
        }
        let now = now();
        let mut todo = Todo {
            id: uuid::Uuid::new_v4().to_string(),
            meta: todo::Meta {
                title: String::new(),
                url: None,
                due: None,
                source: None,
                created_at: now.clone(),
                updated_at: now.clone(),
                done_at: None,
                extra: serde_json::Map::new(),
            },
            body: String::new(),
        };
        todo::apply(&mut todo, fields, &now)?;
        let _guard = self.guard();
        self.write(&todo)?;
        Ok(todo)
    }

    pub fn update(&self, id: &str, fields: &Value) -> Result<Todo, Error> {
        let _guard = self.guard();
        let mut todo = self.read(id)?;
        todo::apply(&mut todo, fields, &now())?;
        self.write(&todo)?;
        Ok(todo)
    }

    pub fn delete(&self, id: &str) -> Result<(), Error> {
        let _guard = self.guard();
        self.read(id)?;
        fs::remove_file(self.path(id)?)?;
        File::open(&self.dir)?.sync_all()?;
        Ok(())
    }

    /// Every to-do file verbatim, read under the writer lock for a consistent backup.
    pub fn snapshot(&self) -> Result<BTreeMap<String, String>, Error> {
        let _guard = self.guard();
        Ok(self
            .files()?
            .into_iter()
            .map(|(id, text)| (format!("{id}.md"), text))
            .collect())
    }
}
