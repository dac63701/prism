use std::path::{Path, PathBuf};

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

    fn safe_path(&self, path: &str) -> Result<PathBuf, std::io::Error> {
        for component in Path::new(path).components() {
            if matches!(component, std::path::Component::ParentDir) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Path traversal detected",
                ));
            }
        }
        Ok(self.root.join(path))
    }

    fn ensure_parent(&self, path: &str) -> Result<(), std::io::Error> {
        if let Some(parent) = self.safe_path(path)?.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl StorageBackend for LocalStorage {
    async fn store(&self, path: &str, data: &[u8]) -> Result<(), std::io::Error> {
        self.ensure_parent(path)?;
        let full = self.safe_path(path)?;
        let tmp = full.with_extension("tmp");
        tokio::fs::write(&tmp, data).await?;
        tokio::fs::rename(&tmp, &full).await?;
        Ok(())
    }

    async fn retrieve(&self, path: &str) -> Result<Vec<u8>, std::io::Error> {
        tokio::fs::read(self.safe_path(path)?).await
    }

    async fn delete(&self, path: &str) -> Result<(), std::io::Error> {
        let full = self.safe_path(path)?;
        if full.exists() {
            tokio::fs::remove_file(&full).await?;
        }
        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool, std::io::Error> {
        Ok(self.safe_path(path)?.exists())
    }
}
