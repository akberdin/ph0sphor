use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    #[default]
    PhosphorGreen,
    AmberCrt,
    IceTerminal,
    MonoLcd,
    HighContrast,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PhosphorGreen => "phosphor-green",
            Self::AmberCrt => "amber-crt",
            Self::IceTerminal => "ice-terminal",
            Self::MonoLcd => "mono-lcd",
            Self::HighContrast => "high-contrast",
        }
    }
}
