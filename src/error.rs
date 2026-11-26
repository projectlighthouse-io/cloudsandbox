use std::fmt;

use route::{Body, IntoResponse, http::{Response, StatusCode}};

#[derive(Debug)]
pub enum AppError {
    Config(String),
    Db(sqlx::Error),
    Routes(route::BuildError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "config: {msg}"),
            Self::Db(e) => write!(f, "database: {e}"),
            Self::Routes(e) => write!(f, "routes: {e}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db(e)
    }
}

impl From<route::BuildError> for AppError {
    fn from(e: route::BuildError) -> Self {
        Self::Routes(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response<Body> {
        tracing::error!("{self}");
        (StatusCode::INTERNAL_SERVER_ERROR, "server error").into_response()
    }
}
