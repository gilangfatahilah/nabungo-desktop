use nabungo_core::PeriodStartDay;
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
pub(crate) const KEY_LOCALE: &str = "ui.locale";
pub(crate) const KEY_PERIOD_START_DAY: &str = "budget.period_start_day";

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
    pub(crate) fn as_str(self) -> &'static str {
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

    /// Hilang/tak valid → tanggal 1.
    pub fn period_start_day(&self) -> Result<PeriodStartDay, AppError> {
        Ok(setting::get_setting(&self.conn, KEY_PERIOD_START_DAY)?
            .and_then(|v| v.parse::<u8>().ok())
            .and_then(|d| PeriodStartDay::new(d).ok())
            .unwrap_or(PeriodStartDay::FIRST))
    }

    pub fn set_period_start_day(&mut self, day: PeriodStartDay) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        setting::set_setting(&tx, KEY_PERIOD_START_DAY, &day.day().to_string())?;
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
    fn period_start_day_default_and_roundtrip() {
        let mut l = Ledger::open_in_memory().unwrap();
        assert_eq!(l.period_start_day().unwrap(), PeriodStartDay::FIRST);
        l.set_period_start_day(PeriodStartDay::new(25).unwrap())
            .unwrap();
        assert_eq!(l.period_start_day().unwrap().day(), 25);
        for bad in ["99", "abc"] {
            l.conn
                .execute(
                    "UPDATE setting SET value = ?1 WHERE key = 'budget.period_start_day'",
                    [bad],
                )
                .unwrap();
            assert_eq!(l.period_start_day().unwrap(), PeriodStartDay::FIRST);
        }
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
