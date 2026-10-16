use nabungo_storage::setting;

use crate::{AppError, Ledger};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    #[default]
    Id,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiSettings {
    pub theme: Theme,
    pub locale: Locale,
}

const KEY_THEME: &str = "ui.theme";
const KEY_LOCALE: &str = "ui.locale";

impl Theme {
    fn as_str(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
            Theme::System => "system",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            "system" => Some(Theme::System),
            _ => None,
        }
    }
}

impl Locale {
    fn as_str(self) -> &'static str {
        match self {
            Locale::Id => "id",
            Locale::En => "en",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "id" => Some(Locale::Id),
            "en" => Some(Locale::En),
            _ => None,
        }
    }
}

impl Ledger {
    /// Nilai hilang/tak dikenal jatuh ke default, tidak pernah error.
    pub fn ui_settings(&self) -> Result<UiSettings, AppError> {
        let theme = setting::get_setting(&self.conn, KEY_THEME)?
            .and_then(|v| Theme::parse(&v))
            .unwrap_or_default();
        let locale = setting::get_setting(&self.conn, KEY_LOCALE)?
            .and_then(|v| Locale::parse(&v))
            .unwrap_or_default();
        Ok(UiSettings { theme, locale })
    }

    pub fn set_theme(&mut self, theme: Theme) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        setting::set_setting(&tx, KEY_THEME, theme.as_str())?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_locale(&mut self, locale: Locale) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        setting::set_setting(&tx, KEY_LOCALE, locale.as_str())?;
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_settings_default_when_empty() {
        let l = Ledger::open_in_memory().unwrap();
        assert_eq!(
            l.ui_settings().unwrap(),
            UiSettings {
                theme: Theme::System,
                locale: Locale::Id
            }
        );
    }

    #[test]
    fn ui_settings_roundtrip() {
        let mut l = Ledger::open_in_memory().unwrap();
        l.set_theme(Theme::Dark).unwrap();
        l.set_locale(Locale::En).unwrap();
        assert_eq!(
            l.ui_settings().unwrap(),
            UiSettings {
                theme: Theme::Dark,
                locale: Locale::En
            }
        );
    }

    #[test]
    fn unknown_setting_value_falls_back_to_default() {
        let l = Ledger::open_in_memory().unwrap();
        l.conn
            .execute(
                "INSERT INTO setting VALUES ('ui.theme','neon'), ('ui.locale','fr')",
                [],
            )
            .unwrap();
        assert_eq!(l.ui_settings().unwrap(), UiSettings::default());
    }
}
