use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Read a file from the filesystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileParams {
    /// Path to the file to read
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub content: String,
    pub size: usize,
}

pub async fn read_file(params: ReadFileParams) -> Result<FileContent, String> {
    let path = PathBuf::from(&params.path);

    if !path.exists() {
        return Err(format!("File not found: {}", params.path));
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let size = content.len();

    Ok(FileContent { content, size })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_file_not_found() {
        let params = ReadFileParams {
            path: "/nonexistent/file.txt".to_string(),
        };

        let result = read_file(params).await;
        assert!(result.is_err());
    }
}
