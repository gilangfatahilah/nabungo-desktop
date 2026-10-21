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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiSettings {
    pub theme: Theme,
    pub locale: Locale,
    /// Menit tanpa aktivitas sebelum app terkunci; hanya berlaku bila DB terenkripsi.
    pub auto_lock_minutes: u8,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            locale: Locale::default(),
            auto_lock_minutes: DEFAULT_AUTO_LOCK_MINUTES,
        }
    }
}

/// Pilihan durasi kunci otomatis (menit).
pub const AUTO_LOCK_CHOICES: [u8; 5] = [1, 5, 10, 15, 30];
const DEFAULT_AUTO_LOCK_MINUTES: u8 = 5;

const KEY_THEME: &str = "ui.theme";
pub(crate) const KEY_LOCALE: &str = "ui.locale";
pub(crate) const KEY_PERIOD_START_DAY: &str = "budget.period_start_day";
const KEY_AUTO_LOCK: &str = "auto_lock_minutes";

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
        let auto_lock_minutes = setting::get_setting(&self.conn, KEY_AUTO_LOCK)?
            .and_then(|v| v.parse::<u8>().ok())
            .filter(|m| AUTO_LOCK_CHOICES.contains(m))
            .unwrap_or(DEFAULT_AUTO_LOCK_MINUTES);
        Ok(UiSettings {
            theme,
            locale,
            auto_lock_minutes,
        })
    }

    pub fn set_auto_lock_minutes(&mut self, minutes: u8) -> Result<(), AppError> {
        if !AUTO_LOCK_CHOICES.contains(&minutes) {
            return Err(AppError::InvalidAutoLock);
        }
        let tx = self.conn.transaction()?;
        setting::set_setting(&tx, KEY_AUTO_LOCK, &minutes.to_string())?;
        tx.commit()?;
        Ok(())
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
                locale: Locale::Id,
                auto_lock_minutes: 5,
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
                locale: Locale::En,
                auto_lock_minutes: 5,
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
    fn auto_lock_default_and_roundtrip() {
        let mut l = Ledger::open_in_memory().unwrap();
        assert_eq!(l.ui_settings().unwrap().auto_lock_minutes, 5);
        l.set_auto_lock_minutes(15).unwrap();
        assert_eq!(l.ui_settings().unwrap().auto_lock_minutes, 15);
        assert!(matches!(
            l.set_auto_lock_minutes(7),
            Err(AppError::InvalidAutoLock)
        ));
        assert_eq!(l.ui_settings().unwrap().auto_lock_minutes, 15);
        l.conn
            .execute(
                "UPDATE setting SET value = '7' WHERE key = 'auto_lock_minutes'",
                [],
            )
            .unwrap();
        assert_eq!(l.ui_settings().unwrap().auto_lock_minutes, 5);
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
