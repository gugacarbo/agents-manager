use agents_manager_core::{
    domain::{SyncApplyRequest, SyncPreviewItem},
    error::CoreError,
};
use std::sync::Mutex;

struct PreviewRecord {
    request: SyncApplyRequest,
    fingerprint: String,
    paths: Vec<String>,
    previews: Vec<SyncPreviewItem>,
}
#[derive(Default)]
pub struct PreviewState(Mutex<Option<PreviewRecord>>);

pub fn store_preview(
    state: &PreviewState,
    request: SyncApplyRequest,
    fingerprint: String,
    paths: Vec<String>,
    previews: Vec<SyncPreviewItem>,
) -> Result<(), CoreError> {
    *state.0.lock().map_err(|e| CoreError::IoError {
        message: e.to_string(),
    })? = Some(PreviewRecord {
        request,
        fingerprint,
        paths,
        previews,
    });
    Ok(())
}

pub fn take_preview(
    state: &PreviewState,
    request: &SyncApplyRequest,
    fingerprint: &str,
    paths: &[String],
) -> Option<Vec<SyncPreviewItem>> {
    let record = state.0.lock().ok()?.take()?;
    if record.request == *request && record.fingerprint == fingerprint && record.paths == paths {
        Some(record.previews)
    } else {
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use agents_manager_core::domain::{AgentKind, ConfigScope, SyncApplyTarget, SyncSelection};

    #[test]
    fn apply_requires_matching_preview_and_consumes_it() {
        let selection = SyncSelection {
            agent: AgentKind::Codex,
            scope: ConfigScope::Global,
            project_id: None,
            copy: None,
        };
        let request = SyncApplyRequest {
            item_id: "item".into(),
            targets: vec![SyncApplyTarget {
                selection: selection.clone(),
                expected_sha256: None,
            }],
        };
        let state = PreviewState::default();
        assert!(take_preview(&state, &request, "source", &[]).is_none());
        store_preview(&state, request.clone(), "source".into(), vec![], vec![]).unwrap();
        assert!(take_preview(&state, &request, "changed", &[]).is_none());
        store_preview(
            &state,
            request.clone(),
            "source".into(),
            vec!["original".into()],
            vec![],
        )
        .unwrap();
        assert!(take_preview(&state, &request, "source", &["other".into()]).is_none());
        store_preview(&state, request.clone(), "source".into(), vec![], vec![]).unwrap();
        let mut changed = request.clone();
        changed.targets[0].expected_sha256 = Some("modified".into());
        assert!(take_preview(&state, &changed, "source", &[]).is_none());
        store_preview(&state, request.clone(), "source".into(), vec![], vec![]).unwrap();
        assert!(take_preview(&state, &request, "source", &[]).is_some());
        assert!(take_preview(&state, &request, "source", &[]).is_none());
    }
}
