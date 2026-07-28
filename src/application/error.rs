use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("requisição inválida: {0}")]
    Validation(&'static str),
    #[error("credenciais inválidas")]
    InvalidCredentials,
    #[error("não autenticado")]
    Unauthorized,
    #[error("acesso negado")]
    Forbidden,
    #[error("muitas tentativas")]
    RateLimited,
    #[error("conflito de dados: {0}")]
    Conflict(&'static str),
    #[error("recurso indisponível")]
    Unavailable,
    #[error("erro interno")]
    Internal,
}
