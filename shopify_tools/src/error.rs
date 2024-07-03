use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShopifyApiError {
    #[error("Could not initialize client: {0}")]
    Initialization(String),
    #[error("Invalid REST request: {0}")]
    RestRequestError(String),
    #[error("Invalid REST response: {0}")]
    RestResponseError(String),
    #[error("Could not deserialize JSON: {0}")]
    JsonError(String),
    #[error("Query failed. Error {status}. {message}")]
    QueryError { status: u16, message: String },
    #[error("Invalid GraphQL query: {0}")]
    InvalidGraphQL(String),
    #[error("GraphQL query failed: {0}")]
    GraphQLError(String),
    #[error("Invalid currency amount: {0}")]
    InvalidCurrencyAmount(String),
}
