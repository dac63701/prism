#[async_trait::async_trait]
pub trait StorageBackend: Send + Sync {
    async fn store(&self, path: &str, data: &[u8]) -> Result<(), std::io::Error>;
    async fn retrieve(&self, path: &str) -> Result<Vec<u8>, std::io::Error>;
    async fn delete(&self, path: &str) -> Result<(), std::io::Error>;
    #[allow(dead_code)]
    async fn exists(&self, path: &str) -> Result<bool, std::io::Error>;
}

pub mod local;
