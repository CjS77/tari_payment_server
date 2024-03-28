use actix_web::{
    error::ResponseError,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use log::error;
use tari_payment_engine::AuthApiError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Could not initialize server. {0}")]
    InitializeError(String),
    #[error("An error occurred on the backend of the server. {0}")]
    BackendError(String),
    #[error("Payload deserialization error")]
    CouldNotDeserializePayload,
    #[error("Auth token signature invalid or not provided")]
    CouldNotDeserializeAuthToken,
    #[error("Could not read request body: {0}")]
    InvalidRequestBody(String),
    #[error("Could not read request path: {0}")]
    InvalidRequestPath(String),
    #[error("An I/O error happened in the server. {0}")]
    IOError(#[from] std::io::Error),
    #[error("Order conversion error. {0}")]
    OrderConversionError(#[from] OrderConversionError),
    #[error("Invalid server configuration. {0}")]
    ConfigurationError(String),
    #[error("UnspecifiedError. {0}")]
    Unspecified(String),
    #[error("Authentication Error. {0}")]
    AuthenticationError(#[from] AuthError),
    #[error("Could not serialize access token. {0}")]
    CouldNotSerializeAccessToken(String),
    #[error("The data was not found. {0}")]
    NoRecordFound(String),
    #[error("Insufficient Permissions. {0}")]
    InsufficientPermissions(String),
}

impl ResponseError for ServerError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidRequestBody(_) => StatusCode::BAD_REQUEST,
            Self::CouldNotDeserializePayload => StatusCode::BAD_REQUEST,
            Self::CouldNotDeserializeAuthToken => StatusCode::BAD_REQUEST,
            Self::AuthenticationError(e) => match e {
                AuthError::InvalidPublicKey => StatusCode::UNAUTHORIZED,
                AuthError::InsufficientPermissions(_) => StatusCode::FORBIDDEN,
                AuthError::ValidationError(_) => StatusCode::UNAUTHORIZED,
                AuthError::PoorlyFormattedToken(_) => StatusCode::BAD_REQUEST,
                AuthError::AccountNotFound => StatusCode::FORBIDDEN,
            },
            Self::InitializeError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::BackendError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::IOError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::OrderConversionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ConfigurationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Unspecified(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::CouldNotSerializeAccessToken(_) => StatusCode::BAD_REQUEST,
            Self::InvalidRequestPath(_) => StatusCode::BAD_REQUEST,
            Self::NoRecordFound(_) => StatusCode::NOT_FOUND,
            ServerError::InsufficientPermissions(_) => StatusCode::FORBIDDEN,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(serde_json::json!({ "error": self.to_string() }).to_string())
    }
}

#[derive(Debug, Error)]
#[error("Could not convert shopify order into a new order. {0}.")]
pub struct OrderConversionError(pub String);

#[derive(Debug, Clone, Error)]
pub enum AuthError {
    #[error("Login token contained an invalid public key.")]
    InvalidPublicKey,
    #[error("Insufficient Permissions. {0}")]
    InsufficientPermissions(String),
    #[error("Login token signature is invalid. {0}")]
    ValidationError(String),
    #[error("Login token is not in the correct format. {0}")]
    PoorlyFormattedToken(String),
    #[error("User account not found.")]
    AccountNotFound,
}

impl From<AuthApiError> for ServerError {
    fn from(e: AuthApiError) -> Self {
        match e {
            AuthApiError::InvalidNonce => Self::AuthenticationError(AuthError::ValidationError(e.to_string())),
            AuthApiError::AddressNotFound => Self::AuthenticationError(AuthError::AccountNotFound),
            AuthApiError::RoleNotAllowed(_) => {
                Self::AuthenticationError(AuthError::InsufficientPermissions(e.to_string()))
            },
            AuthApiError::DatabaseError(e) => Self::BackendError(format!("Database error: {e}")),
            AuthApiError::RoleNotFound => {
                Self::BackendError(format!("Role definitions in Database and Code have diverged. {e}"))
            },
        }
    }
}
