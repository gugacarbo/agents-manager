use crate::{
    domain::{ProjectEntry, ProjectSummary},
    error::CoreError,
};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};

fn entries(registry: &Path) -> Result<Vec<ProjectEntry>, CoreError> {
    match fs::read(registry) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(e.into()),
    }
}
fn summary(entry: ProjectEntry) -> ProjectSummary {
    let available = Path::new(&entry.root_path).is_dir();
    ProjectSummary {
        id: entry.id,
        name: entry.name,
        root_path: entry.root_path,
        added_at: entry.added_at,
        available,
    }
}
pub fn list_projects(registry: &Path) -> Result<Vec<ProjectSummary>, CoreError> {
    Ok(entries(registry)?.into_iter().map(summary).collect())
}
fn store(registry: &Path, projects: &[ProjectEntry]) -> Result<(), CoreError> {
    let parent = registry.parent().ok_or_else(|| CoreError::InvalidPath {
        path: registry.display().to_string(),
    })?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(&serde_json::to_vec_pretty(projects)?)?;
    temp.as_file().sync_all()?;
    temp.persist(registry)
        .map_err(|e| CoreError::from(e.error))?;
    Ok(())
}
pub fn register_project(registry: &Path, root: &Path) -> Result<ProjectSummary, CoreError> {
    let canonical = fs::canonicalize(root).map_err(|_| CoreError::InvalidPath {
        path: root.display().to_string(),
    })?;
    if !canonical.is_dir() {
        return Err(CoreError::InvalidPath {
            path: root.display().to_string(),
        });
    }
    let root_path = canonical
        .to_str()
        .ok_or_else(|| CoreError::InvalidPath {
            path: root.display().to_string(),
        })?
        .to_owned();
    let mut projects = entries(registry)?;
    if let Some(existing) = projects.iter().find(|p| p.root_path == root_path) {
        return Err(CoreError::DuplicateProject {
            id: existing.id.clone(),
            root_path,
        });
    }
    let id = format!("{:x}", Sha256::digest(root_path.as_bytes()));
    let name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&root_path)
        .to_owned();
    let entry = ProjectEntry {
        id,
        name,
        root_path,
        added_at: chrono::Utc::now().to_rfc3339(),
    };
    projects.push(entry.clone());
    store(registry, &projects)?;
    Ok(summary(entry))
}
pub fn remove_project(registry: &Path, id: &str) -> Result<(), CoreError> {
    let mut projects = entries(registry)?;
    let before = projects.len();
    projects.retain(|project| project.id != id);
    if projects.len() == before {
        return Err(CoreError::NotFound {
            target: id.to_owned(),
        });
    }
    store(registry, &projects)
}
