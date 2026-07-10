use std::path::PathBuf;

use super::StorageBackend;

#[derive(Clone)]
pub struct LocalStorage {
    root: PathBuf,
}

impl LocalStorage {
    pub fn new(root: &str) -> Self {
        Self {
            root: PathBuf::from(root),
        }
    }

    fn full_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    fn ensure_parent(&self, path: &str) -> Result<(), std::io::Error> {
        if let Some(parent) = self.full_path(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl StorageBackend for LocalStorage {
    async fn store(&self, path: &str, data: &[u8]) -> Result<(), std::io::Error> {
        self.ensure_parent(path)?;
        let full = self.full_path(path);
        let tmp = full.with_extension("tmp");
        tokio::fs::write(&tmp, data).await?;
        tokio::fs::rename(&tmp, &full).await?;
        Ok(())
    }

    async fn retrieve(&self, path: &str) -> Result<Vec<u8>, std::io::Error> {
        tokio::fs::read(self.full_path(path)).await
    }

    async fn delete(&self, path: &str) -> Result<(), std::io::Error> {
        let full = self.full_path(path);
        if full.exists() {
            tokio::fs::remove_file(&full).await?;
        }
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool, std::io::Error> {
        Ok(self.full_path(path).exists())
    }
}
