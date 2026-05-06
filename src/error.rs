use thiserror::Error;

#[derive(Debug, Error)]
pub enum CzmlError {
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Nyx error: {0}")]
    NyxError(#[from] nyx_space::NyxError),

    #[error("Trajectory contains no states in the requested time window")]
    EmptyTrajectory,
}
