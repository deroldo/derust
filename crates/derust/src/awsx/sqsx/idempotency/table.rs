//! Validation of the (optionally schema-qualified) idempotency table name.
//!
//! The table name is the only value interpolated into SQL, so it is validated against
//! `^[A-Za-z_][A-Za-z0-9_]{0,62}(\.[A-Za-z_][A-Za-z0-9_]{0,62})?$` (no regex dependency).

use super::IdempotencyConfigError;

fn is_valid_identifier(part: &str) -> bool {
    let mut chars = part.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    part.len() <= 63 && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Returns the table name unchanged when it is a valid identifier, or `schema.table`.
pub(crate) fn validate_table_name(table: &str) -> Result<&str, IdempotencyConfigError> {
    let mut parts = table.split('.');
    let first = parts.next().unwrap_or("");
    let second = parts.next();
    let valid = parts.next().is_none()
        && is_valid_identifier(first)
        && second.map(is_valid_identifier).unwrap_or(true);
    if valid {
        Ok(table)
    } else {
        Err(IdempotencyConfigError::InvalidTableName)
    }
}

/// Name used inside the index identifier: the table name with `.` replaced by `_`,
/// limited so that `idx_<..>_by_expires_at` fits in 63 bytes.
pub(crate) fn index_suffix(table: &str) -> String {
    table.replace('.', "_").chars().take(63 - 4 - 14).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_names() {
        assert!(validate_table_name("sqs_idempotency").is_ok());
        assert!(validate_table_name("app.sqs_idempotency").is_ok());
        assert!(validate_table_name("_t1").is_ok());
        assert!(validate_table_name(&"a".repeat(63)).is_ok());
    }

    #[test]
    fn rejects_invalid_names() {
        for name in [
            "x; drop table y",
            "",
            "\"quoted\"",
            "1table",
            "a.b.c",
            ".t",
            "t.",
            "a b",
            "t-1",
            "tabela_ç",
            &"a".repeat(64),
        ] {
            assert_eq!(
                validate_table_name(name),
                Err(IdempotencyConfigError::InvalidTableName),
                "{name:?}"
            );
        }
        let long_schema = format!("{}.t", "a".repeat(64));
        assert!(validate_table_name(&long_schema).is_err());
    }
}
