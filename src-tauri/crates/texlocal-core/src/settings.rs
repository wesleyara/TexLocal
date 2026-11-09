//! Configurações globais do usuário (`settings.json`).

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

const THEMES: [&str; 3] = ["system", "dark", "light"];
const LOCALES: [&str; 3] = ["auto", "pt-BR", "en"];
const TEX_SOURCES: [&str; 4] = ["unset", "system", "managed", "skipped"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub theme: String,
    pub locale: String,
    /// Overrides de atalhos: id do comando -> binding.
    pub keybindings: BTreeMap<String, String>,
    /// Origem do TeX: `unset` (assistente pendente), `system`, `managed` ou `skipped`.
    #[serde(rename = "texSource")]
    pub tex_source: String,
    /// Diretório `bin` do TeX do sistema escolhido.
    #[serde(rename = "texBinDir", skip_serializing_if = "Option::is_none")]
    pub tex_bin_dir: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            locale: "auto".into(),
            keybindings: BTreeMap::new(),
            tex_source: "unset".into(),
            tex_bin_dir: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SettingsError {
    Parse(String),
    InvalidValue { field: &'static str, value: String },
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "settings.json inválido: {e}"),
            Self::InvalidValue { field, value } => {
                write!(f, "valor inválido para {field}: {value}")
            }
        }
    }
}

impl std::error::Error for SettingsError {}

impl Settings {
    pub fn validate(&self) -> Result<(), SettingsError> {
        if !THEMES.contains(&self.theme.as_str()) {
            return Err(SettingsError::InvalidValue {
                field: "theme",
                value: self.theme.clone(),
            });
        }
        if !LOCALES.contains(&self.locale.as_str()) {
            return Err(SettingsError::InvalidValue {
                field: "locale",
                value: self.locale.clone(),
            });
        }
        if !TEX_SOURCES.contains(&self.tex_source.as_str()) {
            return Err(SettingsError::InvalidValue {
                field: "texSource",
                value: self.tex_source.clone(),
            });
        }
        Ok(())
    }

    /// Lê e valida o conteúdo de um `settings.json` (campos ausentes assumem o padrão).
    pub fn from_json(json: &str) -> Result<Self, SettingsError> {
        let settings: Self =
            serde_json::from_str(json).map_err(|e| SettingsError::Parse(e.to_string()))?;
        settings.validate()?;
        Ok(settings)
    }

    pub fn to_json(&self) -> String {
        let mut json = serde_json::to_string_pretty(self).expect("Settings é serializável");
        json.push('\n');
        json
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_object_uses_defaults() {
        assert_eq!(Settings::from_json("{}").unwrap(), Settings::default());
    }

    #[test]
    fn rejects_invalid_theme() {
        let err = Settings::from_json(r#"{"theme":"neon"}"#).unwrap_err();
        assert!(matches!(
            err,
            SettingsError::InvalidValue { field: "theme", .. }
        ));
    }

    #[test]
    fn tex_fields_roundtrip_with_camel_case_names() {
        let json = r#"{"texSource":"system","texBinDir":"/usr/bin"}"#;
        let s = Settings::from_json(json).unwrap();
        assert_eq!(s.tex_source, "system");
        assert_eq!(s.tex_bin_dir.as_deref(), Some("/usr/bin"));
        assert!(s.to_json().contains("texBinDir"));
        assert!(Settings::from_json(r#"{"texSource":"x"}"#).is_err());
        assert_eq!(Settings::default().tex_source, "unset");
    }

    #[test]
    fn rejects_unknown_fields() {
        assert!(matches!(
            Settings::from_json(r#"{"foo":1}"#),
            Err(SettingsError::Parse(_))
        ));
    }

    #[test]
    fn roundtrip_uses_lf_and_trailing_newline() {
        let mut s = Settings::default();
        s.keybindings
            .insert("view.toggleSidebar".into(), "mod+b".into());
        let json = s.to_json();
        assert!(json.ends_with('\n'));
        assert!(!json.contains('\r'));
        assert_eq!(Settings::from_json(&json).unwrap(), s);
    }
}
