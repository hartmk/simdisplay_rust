use crate::model::Layout;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Condvar, Mutex, mpsc},
    thread::{self, JoinHandle},
};
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Workspace {
    pub active: Layout,
    pub library: BTreeMap<String, Layout>,
}
impl Workspace {
    pub fn validate(&self) -> Result<(), String> {
        self.active.validate()?;
        for (name, l) in &self.library {
            if name.encode_utf16().count() > 80 {
                return Err("ชื่อเลย์เอาต์ยาวเกินไป".into());
            }
            l.validate()?;
        }
        Ok(())
    }
    fn parse(s: &str) -> Result<Self, String> {
        let v: serde_json::Value = serde_json::from_str(s).map_err(|e| e.to_string())?;
        let active = Layout::parse(&v.get("active").ok_or("ไม่มี active layout")?.to_string())?;
        let library = v
            .get("library")
            .and_then(|v| v.as_object())
            .ok_or("ไม่มี layout library")?
            .iter()
            .map(|(k, v)| Ok((k.clone(), Layout::parse(&v.to_string())?)))
            .collect::<Result<_, String>>()?;
        let w = Self { active, library };
        w.validate()?;
        Ok(w)
    }
}
pub struct Store {
    path: PathBuf,
    pub error: Option<String>,
}
impl Store {
    pub fn load(path: PathBuf) -> (Self, Workspace) {
        let file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return (Self { path, error: None }, Workspace::default());
            }
            Err(e) => {
                return (
                    Self {
                        path,
                        error: Some(e.to_string()),
                    },
                    Workspace::default(),
                );
            }
        };
        let result = (|| {
            if file.metadata().map_err(|e| e.to_string())?.len() > 64_000_000 {
                return Err("ไฟล์ workspace ใหญ่เกินไป".into());
            }
            let mut s = String::new();
            file.take(64_000_001)
                .read_to_string(&mut s)
                .map_err(|e| e.to_string())?;
            Workspace::parse(&s)
        })();
        match result {
            Ok(w) => (Self { path, error: None }, w),
            Err(e) => (
                Self {
                    path,
                    error: Some(e),
                },
                Workspace::default(),
            ),
        }
    }
    pub fn save(&self, w: &Workspace) -> Result<(), String> {
        if let Some(e) = &self.error {
            return Err(format!("หยุดบันทึกเพื่อรักษาไฟล์เดิม: {e}"));
        }
        w.validate()?;
        let bytes = serde_json::to_vec_pretty(w).map_err(|e| e.to_string())?;
        atomic_write(&self.path, &bytes)
    }
    pub fn reset(&mut self) -> Result<Option<PathBuf>, String> {
        let exists = match std::fs::metadata(&self.path) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(e.to_string()),
        };
        let backup = if exists {
            let p = self
                .path
                .with_extension(format!("backup-{}.json", uuid::Uuid::new_v4()));
            std::fs::copy(&self.path, &p).map_err(|e| e.to_string())?;
            Some(p)
        } else {
            None
        };
        self.error = None;
        Ok(backup)
    }
}
pub fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("ไม่มีโฟลเดอร์ปลายทาง")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut f = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    f.write_all(bytes)
        .and_then(|()| f.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    f.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
#[derive(Default)]
struct Pending {
    workspace: Option<Workspace>,
    reset: bool,
    stop: bool,
}
pub struct SaveResult {
    pub workspace: Workspace,
    pub reset: bool,
    pub outcome: Result<(), String>,
}
pub struct Saver {
    pending: Arc<(Mutex<Pending>, Condvar)>,
    pub results: mpsc::Receiver<SaveResult>,
    worker: Option<JoinHandle<()>>,
}
impl Saver {
    pub fn new(mut store: Store) -> Self {
        let pending = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let p = pending.clone();
        let (tx, results) = mpsc::channel();
        let worker = thread::spawn(move || {
            loop {
                let (lock, cv) = &*p;
                let mut state = lock.lock().unwrap();
                while state.workspace.is_none() && !state.stop && !state.reset {
                    state = cv.wait(state).unwrap();
                }
                let workspace = state.workspace.take();
                let reset = std::mem::take(&mut state.reset);
                let stop = state.stop;
                drop(state);
                if let Some(w) = workspace {
                    let outcome = if reset {
                        store.reset().and_then(|_| store.save(&w))
                    } else {
                        store.save(&w)
                    };
                    let _ = tx.send(SaveResult {
                        workspace: w,
                        reset,
                        outcome,
                    });
                }
                if stop {
                    break;
                }
            }
        });
        Self {
            pending,
            results,
            worker: Some(worker),
        }
    }
    pub fn submit(&self, w: Workspace) {
        let (lock, cv) = &*self.pending;
        lock.lock().unwrap().workspace = Some(w);
        cv.notify_one();
    }
    pub fn reset_and_save(&self, w: Workspace) {
        let (lock, cv) = &*self.pending;
        let mut p = lock.lock().unwrap();
        p.reset = true;
        p.workspace = Some(w);
        cv.notify_one();
    }
}
impl Drop for Saver {
    fn drop(&mut self) {
        let (lock, cv) = &*self.pending;
        lock.lock().unwrap().stop = true;
        cv.notify_one();
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
