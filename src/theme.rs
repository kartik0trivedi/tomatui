use ratatui::style::Color;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub enum ThemeChoice {
    #[default]
    CatppuccinMocha,
    CatppuccinMacchiato,
    CatppuccinFrappe,
    CatppuccinLatte,
    Nord,
    GruvboxDark,
    TokyoNight,
    Dracula,
    SolarizedDark,
    SolarizedLight,
    RosePine,
    OneDark,
    Kanagawa,
    EverforestDark,
    EverforestLight,
    Synthwave84,
    MonokaiPro,
    OledPhosphor,
}

impl ThemeChoice {
    pub fn all() -> &'static [ThemeChoice] {
        &[
            Self::CatppuccinMocha,
            Self::CatppuccinMacchiato,
            Self::CatppuccinFrappe,
            Self::CatppuccinLatte,
            Self::Nord,
            Self::GruvboxDark,
            Self::TokyoNight,
            Self::Dracula,
            Self::SolarizedDark,
            Self::SolarizedLight,
            Self::RosePine,
            Self::OneDark,
            Self::Kanagawa,
            Self::EverforestDark,
            Self::EverforestLight,
            Self::Synthwave84,
            Self::MonokaiPro,
            Self::OledPhosphor,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::CatppuccinMacchiato => "Catppuccin Macchiato",
            Self::CatppuccinFrappe => "Catppuccin Frappé",
            Self::CatppuccinLatte => "Catppuccin Latte",
            Self::Nord => "Nord",
            Self::GruvboxDark => "Gruvbox Dark",
            Self::TokyoNight => "Tokyo Night",
            Self::Dracula => "Dracula",
            Self::SolarizedDark => "Solarized Dark",
            Self::SolarizedLight => "Solarized Light",
            Self::RosePine => "Rose Pine",
            Self::OneDark => "One Dark",
            Self::Kanagawa => "Kanagawa",
            Self::EverforestDark => "Everforest Dark",
            Self::EverforestLight => "Everforest Light",
            Self::Synthwave84 => "Synthwave '84",
            Self::MonokaiPro => "Monokai Pro",
            Self::OledPhosphor => "OLED Phosphor",
        }
    }

    pub fn next(&self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub fn prev(&self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
}

impl std::str::FromStr for ThemeChoice {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let norm = |v: &str| -> String {
            v.trim()
                .to_ascii_lowercase()
                .replace([' ', '-', '_', '\'', '\u{2018}', '\u{2019}'], "")
                .replace('é', "e")
                .replace('\u{0301}', "")
        };
        let wanted = norm(s);
        for choice in Self::all() {
            if wanted == norm(&format!("{:?}", choice)) || wanted == norm(choice.name()) {
                return Ok(*choice);
            }
        }
        Err(())
    }
}

impl<'de> serde::Deserialize<'de> for ThemeChoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(raw.parse::<ThemeChoice>().unwrap_or_default())
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub choice: ThemeChoice,
    pub bg: Color,
    pub fg: Color,
    pub primary: Color,
    pub secondary: Color,
    pub work: Color,
    pub short_break: Color,
    pub long_break: Color,
    pub success: Color,
    pub warning: Color,
    pub border: Color,
    pub muted: Color,
    pub highlight: Color,
}

impl Theme {
    pub fn from_choice_with_transparency(choice: ThemeChoice, transparent: bool) -> Self {
        let mut theme = Self::from_choice(choice);
        if transparent {
            theme.bg = Color::Reset;
        }
        theme
    }

    /// Solid contrasting foreground for inverted primary buttons (stays opaque when bg is Reset).
    pub fn button_fg(&self) -> Color {
        Self::from_choice(self.choice).bg
    }

    pub fn from_choice(choice: ThemeChoice) -> Self {
        match choice {
            ThemeChoice::CatppuccinMocha => Self {
                choice,
                bg: Color::Rgb(30, 30, 46),
                fg: Color::Rgb(205, 214, 244),
                primary: Color::Rgb(137, 180, 250),
                secondary: Color::Rgb(203, 166, 247),
                work: Color::Rgb(243, 139, 168),
                short_break: Color::Rgb(166, 227, 161),
                long_break: Color::Rgb(148, 226, 213),
                success: Color::Rgb(166, 227, 161),
                warning: Color::Rgb(249, 226, 175),
                border: Color::Rgb(69, 71, 90),
                muted: Color::Rgb(108, 112, 134),
                highlight: Color::Rgb(49, 50, 68),
            },
            ThemeChoice::CatppuccinMacchiato => Self {
                choice,
                bg: Color::Rgb(36, 39, 58),
                fg: Color::Rgb(202, 211, 245),
                primary: Color::Rgb(138, 173, 244),
                secondary: Color::Rgb(198, 160, 246),
                work: Color::Rgb(237, 135, 150),
                short_break: Color::Rgb(166, 218, 149),
                long_break: Color::Rgb(139, 213, 202),
                success: Color::Rgb(166, 218, 149),
                warning: Color::Rgb(238, 212, 159),
                border: Color::Rgb(73, 77, 100),
                muted: Color::Rgb(110, 115, 141),
                highlight: Color::Rgb(54, 58, 79),
            },
            ThemeChoice::CatppuccinFrappe => Self {
                choice,
                bg: Color::Rgb(48, 52, 70),
                fg: Color::Rgb(198, 208, 245),
                primary: Color::Rgb(140, 170, 238),
                secondary: Color::Rgb(202, 158, 230),
                work: Color::Rgb(231, 130, 132),
                short_break: Color::Rgb(166, 209, 137),
                long_break: Color::Rgb(129, 200, 190),
                success: Color::Rgb(166, 209, 137),
                warning: Color::Rgb(229, 200, 144),
                border: Color::Rgb(81, 87, 109),
                muted: Color::Rgb(115, 121, 148),
                highlight: Color::Rgb(65, 69, 89),
            },
            ThemeChoice::CatppuccinLatte => Self {
                choice,
                bg: Color::Rgb(239, 241, 245),
                fg: Color::Rgb(76, 79, 105),
                primary: Color::Rgb(30, 102, 245),
                secondary: Color::Rgb(136, 57, 239),
                work: Color::Rgb(210, 15, 57),
                short_break: Color::Rgb(64, 160, 43),
                long_break: Color::Rgb(23, 146, 153),
                success: Color::Rgb(64, 160, 43),
                warning: Color::Rgb(223, 142, 29),
                border: Color::Rgb(204, 208, 218),
                muted: Color::Rgb(140, 143, 161),
                highlight: Color::Rgb(220, 224, 232),
            },
            ThemeChoice::Nord => Self {
                choice,
                bg: Color::Rgb(46, 52, 64),
                fg: Color::Rgb(236, 239, 244),
                primary: Color::Rgb(136, 192, 208),
                secondary: Color::Rgb(129, 161, 193),
                work: Color::Rgb(191, 97, 106),
                short_break: Color::Rgb(163, 190, 140),
                long_break: Color::Rgb(143, 188, 187),
                success: Color::Rgb(163, 190, 140),
                warning: Color::Rgb(235, 203, 139),
                border: Color::Rgb(67, 76, 94),
                muted: Color::Rgb(129, 161, 193),
                highlight: Color::Rgb(59, 66, 82),
            },
            ThemeChoice::GruvboxDark => Self {
                choice,
                bg: Color::Rgb(40, 40, 40),
                fg: Color::Rgb(235, 219, 178),
                primary: Color::Rgb(250, 189, 47),
                secondary: Color::Rgb(254, 128, 25),
                work: Color::Rgb(251, 73, 52),
                short_break: Color::Rgb(184, 187, 38),
                long_break: Color::Rgb(142, 192, 124),
                success: Color::Rgb(184, 187, 38),
                warning: Color::Rgb(250, 189, 47),
                border: Color::Rgb(80, 73, 69),
                muted: Color::Rgb(146, 131, 116),
                highlight: Color::Rgb(60, 56, 54),
            },
            ThemeChoice::TokyoNight => Self {
                choice,
                bg: Color::Rgb(26, 27, 38),
                fg: Color::Rgb(192, 202, 245),
                primary: Color::Rgb(122, 162, 247),
                secondary: Color::Rgb(187, 154, 247),
                work: Color::Rgb(247, 118, 142),
                short_break: Color::Rgb(158, 206, 106),
                long_break: Color::Rgb(115, 218, 202),
                success: Color::Rgb(158, 206, 106),
                warning: Color::Rgb(224, 175, 104),
                border: Color::Rgb(41, 46, 66),
                muted: Color::Rgb(86, 95, 137),
                highlight: Color::Rgb(36, 40, 59),
            },
            ThemeChoice::Dracula => Self {
                choice,
                bg: Color::Rgb(40, 42, 54),
                fg: Color::Rgb(248, 248, 242),
                primary: Color::Rgb(189, 147, 249),
                secondary: Color::Rgb(139, 233, 253),
                work: Color::Rgb(255, 85, 85),
                short_break: Color::Rgb(80, 250, 123),
                long_break: Color::Rgb(255, 184, 108),
                success: Color::Rgb(80, 250, 123),
                warning: Color::Rgb(241, 250, 140),
                border: Color::Rgb(68, 71, 90),
                muted: Color::Rgb(98, 114, 164),
                highlight: Color::Rgb(68, 71, 90),
            },
            ThemeChoice::SolarizedDark => Self {
                choice,
                bg: Color::Rgb(0, 43, 54),
                fg: Color::Rgb(147, 161, 161),
                primary: Color::Rgb(38, 139, 210),
                secondary: Color::Rgb(42, 161, 152),
                work: Color::Rgb(220, 50, 47),
                short_break: Color::Rgb(133, 153, 0),
                long_break: Color::Rgb(108, 113, 196),
                success: Color::Rgb(133, 153, 0),
                warning: Color::Rgb(181, 137, 0),
                border: Color::Rgb(7, 54, 66),
                muted: Color::Rgb(88, 110, 117),
                highlight: Color::Rgb(7, 54, 66),
            },
            ThemeChoice::SolarizedLight => Self {
                choice,
                bg: Color::Rgb(253, 246, 227),
                fg: Color::Rgb(88, 110, 117),
                primary: Color::Rgb(38, 139, 210),
                secondary: Color::Rgb(108, 113, 196),
                work: Color::Rgb(220, 50, 47),
                short_break: Color::Rgb(133, 153, 0),
                long_break: Color::Rgb(211, 54, 130),
                success: Color::Rgb(133, 153, 0),
                warning: Color::Rgb(181, 137, 0),
                border: Color::Rgb(238, 232, 213),
                muted: Color::Rgb(147, 161, 161),
                highlight: Color::Rgb(238, 232, 213),
            },
            ThemeChoice::RosePine => Self {
                choice,
                bg: Color::Rgb(25, 23, 36),
                fg: Color::Rgb(224, 222, 244),
                primary: Color::Rgb(156, 207, 216),
                secondary: Color::Rgb(196, 167, 231),
                work: Color::Rgb(235, 111, 146),
                short_break: Color::Rgb(49, 116, 143),
                long_break: Color::Rgb(235, 188, 186),
                success: Color::Rgb(49, 116, 143),
                warning: Color::Rgb(246, 193, 119),
                border: Color::Rgb(38, 35, 58),
                muted: Color::Rgb(110, 106, 134),
                highlight: Color::Rgb(38, 35, 58),
            },
            ThemeChoice::OneDark => Self {
                choice,
                bg: Color::Rgb(40, 44, 52),
                fg: Color::Rgb(171, 178, 191),
                primary: Color::Rgb(97, 175, 239),
                secondary: Color::Rgb(198, 120, 221),
                work: Color::Rgb(224, 108, 117),
                short_break: Color::Rgb(152, 195, 121),
                long_break: Color::Rgb(86, 182, 194),
                success: Color::Rgb(152, 195, 121),
                warning: Color::Rgb(229, 192, 123),
                border: Color::Rgb(62, 68, 82),
                muted: Color::Rgb(92, 99, 112),
                highlight: Color::Rgb(53, 59, 69),
            },
            ThemeChoice::Kanagawa => Self {
                choice,
                bg: Color::Rgb(31, 31, 40),
                fg: Color::Rgb(220, 215, 186),
                primary: Color::Rgb(126, 156, 216),
                secondary: Color::Rgb(149, 127, 184),
                work: Color::Rgb(228, 104, 118),
                short_break: Color::Rgb(118, 148, 106),
                long_break: Color::Rgb(106, 149, 137),
                success: Color::Rgb(118, 148, 106),
                warning: Color::Rgb(255, 160, 102),
                border: Color::Rgb(54, 54, 70),
                muted: Color::Rgb(114, 113, 105),
                highlight: Color::Rgb(42, 42, 55),
            },
            ThemeChoice::EverforestDark => Self {
                choice,
                bg: Color::Rgb(45, 53, 59),
                fg: Color::Rgb(211, 198, 170),
                primary: Color::Rgb(127, 187, 179),
                secondary: Color::Rgb(214, 153, 182),
                work: Color::Rgb(230, 126, 128),
                short_break: Color::Rgb(167, 192, 128),
                long_break: Color::Rgb(131, 192, 146),
                success: Color::Rgb(167, 192, 128),
                warning: Color::Rgb(219, 188, 127),
                border: Color::Rgb(71, 82, 88),
                muted: Color::Rgb(133, 146, 137),
                highlight: Color::Rgb(52, 63, 68),
            },
            ThemeChoice::EverforestLight => Self {
                choice,
                bg: Color::Rgb(253, 246, 227),
                fg: Color::Rgb(92, 106, 114),
                primary: Color::Rgb(58, 148, 134),
                secondary: Color::Rgb(223, 105, 186),
                work: Color::Rgb(248, 85, 82),
                short_break: Color::Rgb(141, 161, 1),
                long_break: Color::Rgb(53, 167, 124),
                success: Color::Rgb(141, 161, 1),
                warning: Color::Rgb(223, 160, 0),
                border: Color::Rgb(234, 228, 203),
                muted: Color::Rgb(147, 170, 159),
                highlight: Color::Rgb(234, 228, 203),
            },
            ThemeChoice::Synthwave84 => Self {
                choice,
                bg: Color::Rgb(38, 35, 53),
                fg: Color::Rgb(240, 239, 241),
                primary: Color::Rgb(54, 249, 246),
                secondary: Color::Rgb(255, 126, 219),
                work: Color::Rgb(254, 68, 80),
                short_break: Color::Rgb(114, 241, 184),
                long_break: Color::Rgb(255, 139, 57),
                success: Color::Rgb(114, 241, 184),
                warning: Color::Rgb(254, 222, 93),
                border: Color::Rgb(73, 67, 99),
                muted: Color::Rgb(132, 139, 189),
                highlight: Color::Rgb(52, 41, 79),
            },
            ThemeChoice::MonokaiPro => Self {
                choice,
                bg: Color::Rgb(45, 42, 46),
                fg: Color::Rgb(252, 252, 250),
                primary: Color::Rgb(120, 220, 232),
                secondary: Color::Rgb(171, 157, 242),
                work: Color::Rgb(255, 97, 136),
                short_break: Color::Rgb(169, 220, 118),
                long_break: Color::Rgb(252, 152, 103),
                success: Color::Rgb(169, 220, 118),
                warning: Color::Rgb(255, 216, 102),
                border: Color::Rgb(64, 62, 65),
                muted: Color::Rgb(114, 112, 114),
                highlight: Color::Rgb(64, 62, 65),
            },
            ThemeChoice::OledPhosphor => Self {
                choice,
                bg: Color::Rgb(0, 0, 0),
                fg: Color::Rgb(51, 255, 102),
                primary: Color::Rgb(0, 255, 102),
                secondary: Color::Rgb(0, 204, 85),
                work: Color::Rgb(255, 51, 51),
                short_break: Color::Rgb(51, 255, 102),
                long_break: Color::Rgb(0, 255, 255),
                success: Color::Rgb(51, 255, 102),
                warning: Color::Rgb(255, 255, 51),
                border: Color::Rgb(26, 51, 26),
                muted: Color::Rgb(26, 102, 42),
                highlight: Color::Rgb(10, 31, 10),
            },
        }
    }
}
