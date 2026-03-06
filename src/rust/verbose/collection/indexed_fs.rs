use std::fs;
use std::path::{Path, PathBuf};

pub trait FsElement {
    fn get_saved_change_date(&self) -> u128;
    fn get_current_change_date(&self) -> Option<u128>;
    fn get_path(&self) -> &Path;

    fn is_exists(&self) -> bool;

    fn recheck_needed(&self) -> bool;
}
#[derive(Clone)]
pub struct IndexedFile {
    path: PathBuf,
    saved_change_date: u128,
}

impl IndexedFile {
    pub fn new(path: PathBuf) -> Option<Self> {
        if path.exists() && path.is_file(){
            let change_date = path.metadata().ok()?.modified().ok()?.elapsed().ok()?.as_millis();
            Some(IndexedFile { path, saved_change_date: change_date })
        } else {
            None
        }

    }
}

impl FsElement for IndexedFile {
    fn get_saved_change_date(&self) -> u128 {
        self.saved_change_date
    }

    fn get_current_change_date(&self) -> Option<u128> {
        Some(
            self.path
                .metadata()
                .ok()?
                .modified()
                .ok()?
                .elapsed()
                .ok()?
                .as_millis(),
        )
    }

    fn get_path(&self) -> &Path {
        self.path.as_path()
    }

    fn is_exists(&self) -> bool {
        self.path.exists()
    }

    fn recheck_needed(&self) -> bool {
        let res = self.get_current_change_date();
        if res.is_none() {
            return true;
        }
        if self.saved_change_date != res.unwrap() {
            return true;
        }
        return false;
    }
}
#[derive(Clone)]
pub struct IndexedDirectory {
    path: PathBuf,
    children: Vec<IndexedFile>,
    children_directories: Vec<PathBuf>,
    saved_change_date: u128,
}

impl IndexedDirectory {
    pub fn new(path: PathBuf, indexed_directories: &mut Vec<IndexedDirectory>) -> Option<Self> {
        if path.exists() && path.is_dir() {
            let change_date = path
                .metadata()
                .ok()?
                .modified()
                .ok()?
                .elapsed()
                .ok()?
                .as_millis();
            let mut children_dirs = Vec::new();
            let mut children = Vec::new();
            for entry in fs::read_dir(path.clone()).ok()? {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        children_dirs.push(path.clone());
                        if let Some(res) = IndexedDirectory::new(path, indexed_directories) {
                            indexed_directories.push(res);
                        }
                    } else {
                        if let Some(file) = IndexedFile::new(path) {
                            children.push(file);
                        }
                    }
                }
            }
            return Some(Self {
                path,
                children,
                children_directories: children_dirs,
                saved_change_date: change_date,
            });
        }
        None
    }
}

impl FsElement for IndexedDirectory {
    fn get_saved_change_date(&self) -> u128 {
        self.saved_change_date
    }

    fn get_current_change_date(&self) -> Option<u128> {
        Some(
            self.path
                .metadata()
                .ok()?
                .modified()
                .ok()?
                .elapsed()
                .ok()?
                .as_millis(),
        )
    }

    fn get_path(&self) -> &Path {
        self.path.as_path()
    }

    fn is_exists(&self) -> bool {
        self.path.exists()
    }

    fn recheck_needed(&self) -> bool {
        let res = self.get_current_change_date();
        if res.is_none() {
            return true;
        }
        if self.saved_change_date != res.unwrap() {
            return true;
        }
        self.children.iter().all(|e| e.recheck_needed())
    }
}
