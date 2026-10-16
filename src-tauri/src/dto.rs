//! DTO serde/specta untuk command. Tanpa logika bisnis.

use nabungo_app::{Locale, Theme, UiSettings};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub portable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum ThemeDto {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "lowercase")]
pub enum LocaleDto {
    Id,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
pub struct UiSettingsDto {
    pub theme: ThemeDto,
    pub locale: LocaleDto,
}

impl From<Theme> for ThemeDto {
    fn from(t: Theme) -> Self {
        match t {
            Theme::Light => ThemeDto::Light,
            Theme::Dark => ThemeDto::Dark,
            Theme::System => ThemeDto::System,
        }
    }
}

impl From<ThemeDto> for Theme {
    fn from(t: ThemeDto) -> Self {
        match t {
            ThemeDto::Light => Theme::Light,
            ThemeDto::Dark => Theme::Dark,
            ThemeDto::System => Theme::System,
        }
    }
}

impl From<Locale> for LocaleDto {
    fn from(l: Locale) -> Self {
        match l {
            Locale::Id => LocaleDto::Id,
            Locale::En => LocaleDto::En,
        }
    }
}

impl From<LocaleDto> for Locale {
    fn from(l: LocaleDto) -> Self {
        match l {
            LocaleDto::Id => Locale::Id,
            LocaleDto::En => Locale::En,
        }
    }
}

impl From<UiSettings> for UiSettingsDto {
    fn from(s: UiSettings) -> Self {
        Self {
            theme: s.theme.into(),
            locale: s.locale.into(),
        }
    }
}
