//! Index command - build codebase index with tree-sitter

use anyhow::Result;
use std::path::Path;
use crate::config::Config;
use crate::index;

pub async fn run(config: Config, path: Option<&str>, force: bool) -> Result<()> {
    let path = Path::new(path.unwrap_or("."));

    // Run indexing with beautiful UI
    let _result = index::index_directory(path, force, config.verbose).await?;

    // Return success even if some files were skipped
    Ok(())
}
