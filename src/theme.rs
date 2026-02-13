use ratatui::style::Color;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub fg: Color,
    pub bg: Color,
    pub dim: Color,
    pub border: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub positive: Color,
    pub negative: Color,
    pub accent: Color,
    pub input_accent: Color,
    pub title: Color,
    pub error: Color,
}

impl Default for Theme {
    fn default() -> Self {
        dark()
    }
}

pub fn by_name(name: &str) -> Theme {
    match name {
        "dark" => dark(),
        "dark-blue" => dark_blue(),
        "dark-green" => dark_green(),
        "dark-red" => dark_red(),
        "dark-violet" => dark_violet(),
        "dark-gray" => dark_gray(),
        "solarized-dark" => solarized_dark(),
        "solarized-light" => solarized_light(),
        "light" => light(),
        "bubblegum" => bubblegum(),
        "no-color" => no_color(),
        _ => dark(),
    }
}

pub const THEME_NAMES: &[&str] = &[
    "dark",
    "dark-blue",
    "dark-green",
    "dark-red",
    "dark-violet",
    "dark-gray",
    "solarized-dark",
    "solarized-light",
    "light",
    "bubblegum",
    "no-color",
];

// -- Themes --

pub fn dark() -> Theme {
    Theme {
        fg: Color::Indexed(253),        // bright white
        bg: Color::Reset,
        dim: Color::Indexed(243),       // mid gray
        border: Color::Indexed(240),
        highlight_bg: Color::Indexed(237),
        highlight_fg: Color::Indexed(255),
        positive: Color::Indexed(46),   // vivid green
        negative: Color::Indexed(196),  // vivid red
        accent: Color::Indexed(81),     // sky cyan
        input_accent: Color::Indexed(220), // gold
        title: Color::Indexed(255),
        error: Color::Indexed(196),
    }
}

pub fn dark_blue() -> Theme {
    Theme {
        fg: Color::Indexed(153),        // pale blue-white
        bg: Color::Reset,
        dim: Color::Indexed(60),        // muted blue-gray
        border: Color::Indexed(24),     // dark blue
        highlight_bg: Color::Indexed(17), // deep navy
        highlight_fg: Color::Indexed(231),
        positive: Color::Indexed(49),   // aquamarine
        negative: Color::Indexed(203),  // salmon red
        accent: Color::Indexed(39),     // dodger blue
        input_accent: Color::Indexed(117), // light blue
        title: Color::Indexed(75),      // cornflower blue
        error: Color::Indexed(203),
    }
}

pub fn dark_green() -> Theme {
    Theme {
        fg: Color::Indexed(194),        // honeydew (pale green-white)
        bg: Color::Reset,
        dim: Color::Indexed(65),        // dark sea green
        border: Color::Indexed(22),     // dark green
        highlight_bg: Color::Indexed(22),
        highlight_fg: Color::Indexed(255),
        positive: Color::Indexed(82),   // bright chartreuse
        negative: Color::Indexed(209),  // coral
        accent: Color::Indexed(120),    // light green
        input_accent: Color::Indexed(156), // pale green
        title: Color::Indexed(46),      // pure green
        error: Color::Indexed(209),
    }
}

pub fn dark_red() -> Theme {
    Theme {
        fg: Color::Indexed(224),        // misty rose (pale pink-white)
        bg: Color::Reset,
        dim: Color::Indexed(95),        // dark rosy brown
        border: Color::Indexed(52),     // dark red
        highlight_bg: Color::Indexed(52),
        highlight_fg: Color::Indexed(255),
        positive: Color::Indexed(107),  // olive green
        negative: Color::Indexed(197),  // deep pink
        accent: Color::Indexed(210),    // light salmon
        input_accent: Color::Indexed(216), // peach
        title: Color::Indexed(196),     // red
        error: Color::Indexed(197),
    }
}

pub fn dark_violet() -> Theme {
    Theme {
        fg: Color::Indexed(225),        // lavender blush
        bg: Color::Reset,
        dim: Color::Indexed(97),        // medium purple dim
        border: Color::Indexed(54),     // dark purple
        highlight_bg: Color::Indexed(53),
        highlight_fg: Color::Indexed(255),
        positive: Color::Indexed(156),  // pale green
        negative: Color::Indexed(211),  // hot pink light
        accent: Color::Indexed(177),    // orchid
        input_accent: Color::Indexed(183), // plum
        title: Color::Indexed(141),     // medium purple
        error: Color::Indexed(211),
    }
}

pub fn dark_gray() -> Theme {
    Theme {
        fg: Color::Indexed(250),        // gray80
        bg: Color::Reset,
        dim: Color::Indexed(240),       // gray50
        border: Color::Indexed(236),    // gray20
        highlight_bg: Color::Indexed(236),
        highlight_fg: Color::Indexed(255),
        positive: Color::Indexed(108),  // dark sea green
        negative: Color::Indexed(138),  // rosy brown
        accent: Color::Indexed(247),    // lighter gray
        input_accent: Color::Indexed(252),
        title: Color::Indexed(255),
        error: Color::Indexed(138),
    }
}

pub fn solarized_dark() -> Theme {
    // base03=#002b36 base02=#073642 base01=#586e75 base0=#839496 base1=#93a1a1
    // yellow=#b58900 orange=#cb4b16 red=#dc322f green=#859900 cyan=#2aa198 blue=#268bd2 violet=#6c71c4
    Theme {
        fg: Color::Indexed(246),        // base0 #839496
        bg: Color::Reset,
        dim: Color::Indexed(240),       // base01 #586e75
        border: Color::Indexed(23),     // base02 #073642
        highlight_bg: Color::Indexed(23),
        highlight_fg: Color::Indexed(230), // base3 #fdf6e3
        positive: Color::Indexed(64),   // green #859900
        negative: Color::Indexed(160),  // red #dc322f
        accent: Color::Indexed(37),     // cyan #2aa198
        input_accent: Color::Indexed(136), // yellow #b58900
        title: Color::Indexed(33),      // blue #268bd2
        error: Color::Indexed(166),     // orange #cb4b16
    }
}

pub fn solarized_light() -> Theme {
    Theme {
        fg: Color::Indexed(240),        // base01 #586e75
        bg: Color::Indexed(230),        // base3 #fdf6e3 (cream)
        dim: Color::Indexed(245),       // base1 #93a1a1
        border: Color::Indexed(187),    // base2 #eee8d5
        highlight_bg: Color::Indexed(187),
        highlight_fg: Color::Indexed(235), // base02
        positive: Color::Indexed(64),   // green
        negative: Color::Indexed(160),  // red
        accent: Color::Indexed(33),     // blue
        input_accent: Color::Indexed(136), // yellow
        title: Color::Indexed(37),      // cyan
        error: Color::Indexed(166),     // orange
    }
}

pub fn light() -> Theme {
    Theme {
        fg: Color::Indexed(234),        // near black
        bg: Color::Indexed(231),        // white
        dim: Color::Indexed(246),       // mid gray
        border: Color::Indexed(251),    // light gray
        highlight_bg: Color::Indexed(253),
        highlight_fg: Color::Indexed(232),
        positive: Color::Indexed(28),   // dark green
        negative: Color::Indexed(124),  // dark red
        accent: Color::Indexed(25),     // dark blue
        input_accent: Color::Indexed(130), // dark orange
        title: Color::Indexed(232),     // black
        error: Color::Indexed(124),
    }
}

pub fn bubblegum() -> Theme {
    Theme {
        fg: Color::Indexed(225),        // light pink white
        bg: Color::Reset,
        dim: Color::Indexed(176),       // plum
        border: Color::Indexed(213),    // pink
        highlight_bg: Color::Indexed(201), // magenta
        highlight_fg: Color::Indexed(231),
        positive: Color::Indexed(49),   // mint
        negative: Color::Indexed(197),  // hot pink
        accent: Color::Indexed(123),    // aqua
        input_accent: Color::Indexed(219), // light pink
        title: Color::Indexed(213),     // pink
        error: Color::Indexed(197),
    }
}

pub fn no_color() -> Theme {
    Theme {
        fg: Color::Reset,
        bg: Color::Reset,
        dim: Color::Reset,
        border: Color::Reset,
        highlight_bg: Color::Reset,
        highlight_fg: Color::Reset,
        positive: Color::Reset,
        negative: Color::Reset,
        accent: Color::Reset,
        input_accent: Color::Reset,
        title: Color::Reset,
        error: Color::Reset,
    }
}
