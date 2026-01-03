use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct Storage {
    root_dir: PathBuf,
}

impl Storage {
    pub async fn new(root_dir: impl AsRef<Path>) -> Result<Self> {
        let root_dir = root_dir.as_ref().to_path_buf();
        fs::create_dir_all(&root_dir).await?;
        Ok(Self { root_dir })
    }

    pub async fn store(&self, data: &[u8]) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hex::encode(hasher.finalize());

        let path = self.root_dir.join(&hash);
        if !path.exists() {
            let mut file = fs::File::create(&path).await?;
            file.write_all(data).await?;
        }

        Ok(hash)
    }

    pub async fn retrieve(&self, hash: &str) -> Result<Option<Vec<u8>>> {
        let path = self.root_dir.join(hash);
        if path.exists() {
            let data = fs::read(path).await?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }
}
