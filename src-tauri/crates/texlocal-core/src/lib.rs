//! Tipos e utilitários compartilhados do TexLocal.

/// Versão do TexLocal (sincronizada pelo release-please).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver() {
        assert_eq!(VERSION.split('.').count(), 3);
    }
}
