use thiserror::Error;

const USERNAME_MIN_LEN: usize = 3;
const USERNAME_MAX_LEN: usize = 32;
const PASSWORD_MIN_LEN: usize = 12;
const PASSWORD_MAX_LEN: usize = 128;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UserValidationError {
    #[error("username deve ter entre {USERNAME_MIN_LEN} e {USERNAME_MAX_LEN} caracteres")]
    InvalidUsernameLength,
    #[error("username deve conter apenas letras, números, ponto, hífen ou underscore")]
    InvalidUsernameCharacters,
    #[error("senha deve ter entre {PASSWORD_MIN_LEN} e {PASSWORD_MAX_LEN} caracteres")]
    InvalidPasswordLength,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedUsername(String);

impl NormalizedUsername {
    pub fn parse(input: &str) -> Result<Self, UserValidationError> {
        let normalized = input.trim().to_lowercase();
        let len = normalized.chars().count();

        if !(USERNAME_MIN_LEN..=USERNAME_MAX_LEN).contains(&len) {
            return Err(UserValidationError::InvalidUsernameLength);
        }

        if !normalized.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        }) {
            return Err(UserValidationError::InvalidUsernameCharacters);
        }

        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn validate_password(password: &str) -> Result<(), UserValidationError> {
    let len = password.chars().count();

    if !(PASSWORD_MIN_LEN..=PASSWORD_MAX_LEN).contains(&len) {
        return Err(UserValidationError::InvalidPasswordLength);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{NormalizedUsername, UserValidationError, validate_password};

    #[test]
    fn normalizes_username() {
        let username = NormalizedUsername::parse("  Maicom.Dev_01 ").unwrap();

        assert_eq!(username.as_str(), "maicom.dev_01");
    }

    #[test]
    fn rejects_unsafe_username_characters() {
        let error = NormalizedUsername::parse("maicom<script>").unwrap_err();

        assert_eq!(error, UserValidationError::InvalidUsernameCharacters);
    }

    #[test]
    fn rejects_short_password() {
        let error = validate_password("curta").unwrap_err();

        assert_eq!(error, UserValidationError::InvalidPasswordLength);
    }
}
