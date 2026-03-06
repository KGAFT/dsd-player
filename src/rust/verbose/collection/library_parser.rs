use crate::verbose::collection::album::Album;

use std::path::{Path, PathBuf};
use crate::verbose::collection::indexed_fs::{FsElement, IndexedDirectory};

pub struct LibraryParser {}

impl LibraryParser {
    pub fn iterate_directory(
        base_path: PathBuf,
        index: &mut Vec<IndexedDirectory>,
        albums_output: &mut Vec<Album>,
    ) {
    }

    fn check_if_path_in_index(
        base_path: PathBuf,
        index: &Vec<IndexedDirectory>,
    ) -> Option<&IndexedDirectory> {
        Some(index.iter().find(|i| i.get_path() == base_path)?)
    }
}
