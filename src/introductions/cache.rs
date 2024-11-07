use std::fs::File;
use std::path::PathBuf;
use std::time::SystemTime;

struct Mp3sCache {
    path: PathBuf,
}

impl Mp3sCache {
    pub fn new(path: &str) -> Self {
        let path = PathBuf::from(path);
        std::fs::create_dir_all(&path).expect("Should create cache directory if it doesn't exist");
        Mp3sCache { path }
    }

    pub fn get(&self, id: &str) -> Option<PathBuf> {
        let file_path = self.path.as_path().with_file_name(id);

        if !file_path.exists() {
            return None;
        }

        // use last modified metadata to record cache hit timestamps
        // file.set_modified(SystemTime::now()).unwrap();
        File::open(file_path.as_path())
            .expect("Should have permission to open audio files")
            .set_modified(SystemTime::now())
            .expect("Cached file's last modified timestamp must be updated");

        Some(file_path)
    }

    pub fn add(&self, id: &str) {
        todo!("add file to cache");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: consider using tempfile for filesystem tests

    #[tokio::test]
    async fn check_last_modified_updated_on_cache_hit() {
        let cache = Mp3sCache::new(".tmp");

        todo!("create a temporary file and set last modified to 0");
        let now = SystemTime::now();

        if (let Some(file) = cache.get("test_file_1")) {
            todo!("Check last modified timestamp");
        }

        todo!("check that last modified was updated");
    }
}
