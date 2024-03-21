use actix_web::{
    error::ResponseError,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use log::error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Payload deserialization error")]
    CouldNotDeserializePayload,
    #[error("Could not read request body: {0}")]
    InvalidRequestBody(String),
    #[error("Could not deliver message because the inbox is full")]
    MailboxFull,
    #[error("Could not deliver message because the mailbox has closed")]
    MailboxClosed,
    #[error("An I/O error happened in the server. {0}")]
    IOError(#[from] std::io::Error),
    #[error("Order conversion error. {0}")]
    OrderConversionError(#[from] OrderConversionError),
    #[error("Invalid server configuration. {0}")]
    ConfigurationError(String),
    #[error("UnspecifiedError. {0}")]
    Unspecified(String),
}

impl ResponseError for ServerError {
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::InvalidRequestBody(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }
}

#[derive(Debug, Error)]
#[error("Could not convert shopify order into a new order. {0}.")]
pub struct OrderConversionError(pub String);
