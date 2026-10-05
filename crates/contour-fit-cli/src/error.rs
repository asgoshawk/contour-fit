#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("{0}")]
    Input(String),
    #[error("{0}")]
    Resource(String),
    #[error(transparent)]
    Fit(#[from] contour_fit_core::FitError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
impl Error {
    pub fn code(&self) -> u8 {
        match self {
            Self::Input(_) => 2,
            Self::Fit(
                contour_fit_core::FitError::InvalidOptions(_)
                | contour_fit_core::FitError::InvalidRaster(_)
                | contour_fit_core::FitError::NoForeground,
            ) => 2,
            Self::Fit(contour_fit_core::FitError::Quality(_)) => 3,
            _ => 4,
        }
    }
}
