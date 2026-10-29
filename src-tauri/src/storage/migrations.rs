#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        assert_eq!(MIGRATIONS.validate(), Ok(()));
    }
}
