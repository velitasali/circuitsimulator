/// Standard border and outline stroke width for all components (in scene pixels).
pub const COMPONENT_BORDER_WIDTH: f64 = 1.0;

/// Standard wire stroke width (in scene pixels).
pub const WIRE_WIDTH: f64 = 1.0;

/// Bus wire stroke width (in scene pixels).
pub const BUS_WIRE_WIDTH: f64 = 3.5;

/// Translucency alpha for component body fills (matching C++ ColorTheme::ComponentFillAlpha: 90 / 255).
pub const COMPONENT_FILL_ALPHA: f64 = 90.0 / 255.0; // ~0.3529

/// Translucency alpha for component symbols (matching C++ ColorTheme::ComponentSymbolAlpha: 55 / 255).
pub const COMPONENT_SYMBOL_ALPHA: f64 = 55.0 / 255.0; // ~0.2157

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorId {
    ViewportBackground,
    CanvasBackground,
    CanvasGrid,
    CanvasGridDots,
    WireDefault,
    WireHigh,
    WireLow,
    WireOpen,
    WireInput,
    ComponentBody,
    ComponentText,
    ComponentIdText,
    ComponentValText,
    ComponentBorder,
    MeterDisplayText,
    ShapeBody,
    PinHigh,
    PinLow,
    PinOpenHigh,
    PinOpenLow,
    PinInputHigh,
    PinInputLow,
    PinOutHigh,
    PinOutLow,
    ItemHovered,
    TunnelColor1,
    TunnelColor2,
    TunnelColor3,
    TreeItemBg1,
    TreeItemBg2,
    TreeItemText1,
    TreeItemText2,
    ListItemBg1,
    ListItemBg2,
    ListItemText1,
    ListItemText2,
    PropLabelText,
    PropHeaderText,
    MemValText,
    MemTypeText,
    MemDataText,
    MemDataBg,
    TerminalRxBg,
    TerminalRxText,
    TerminalTxBg,
    TerminalTxText,
    MsgOkBg,
    MsgOkText,
    MsgWarnBg,
    MsgWarnText,
    MsgErrorBg,
    MsgErrorText,
    MsgDebugPausedBg,
    MsgDebugPausedText,
    MsgDebugRunningBg,
    MsgDebugRunningText,
    CoordsLabelText,
    SimPauseTint,
    ToggleButtonBg,
    McuLabelBg,
    McuLabelText,
    McuLabelBorder,
    McuPcVal1Text,
    McuPcVal2Text,
    ChipBody,
    ChipLabelText,
    EditorText,
    EditorCurrentLine,
    EditorDebugLine,
    EditorGutterNum,
    EditorGutterActiveNum,
    EditorGutterBorder,
    EditorSpace,
    EditorFound,
    EditorErrorUnderline,
    EditorWarnUnderline,
    EditorInfoUnderline,
    EditorCodeBg,
    EditorCodeBorder,
    EditorCodeText,
    LogStdout,
    LogStderr,
    LogErrorText,
    LogWarnText,
    LogInfoText,
    LogSuccessText,
    ScopeChannel1,
    ScopeChannel2,
    ScopeChannel3,
    ScopeChannel4,
    LaChannel1,
    LaChannel2,
    LaChannel3,
    LaChannel4,
    LaChannel5,
    LaChannel6,
    LaChannel7,
    LaChannel8,
}

pub struct ColorTheme;

impl ColorTheme {
    /// Returns the RGBA components `(r, g, b, a)` for a given color ID.
    pub fn get_rgba(c: ColorId, dark: bool) -> (u8, u8, u8, u8) {
        if !dark {
            match c {
                ColorId::ViewportBackground => (175, 175, 180, 255),
                ColorId::CanvasBackground => (245, 248, 250, 255),
                ColorId::CanvasGrid => (40, 50, 70, 75),
                ColorId::CanvasGridDots => (40, 50, 70, 40),
                ColorId::WireDefault => (40, 160, 80, 255),
                ColorId::WireHigh => (240, 80, 80, 255),
                ColorId::WireLow => (80, 80, 240, 255),
                ColorId::WireOpen => (80, 90, 110, 255),
                ColorId::WireInput => (80, 120, 240, 255),
                ColorId::ComponentBody => (255, 255, 255, 255),
                ColorId::ComponentText => (0, 0, 0, 255),
                ColorId::ComponentIdText => (0, 0, 139, 255),
                ColorId::ComponentValText => (139, 0, 0, 255),
                ColorId::ComponentBorder => (0, 0, 0, 255),
                ColorId::MeterDisplayText => (184, 134, 11, 255),
                ColorId::ShapeBody => (128, 128, 128, 255),
                ColorId::PinHigh => (240, 80, 80, 255),
                ColorId::PinLow => (80, 120, 240, 255),
                ColorId::PinOpenHigh => (60, 160, 60, 255),
                ColorId::PinOpenLow => (80, 120, 240, 255),
                ColorId::PinInputHigh => (240, 160, 80, 255),
                ColorId::PinInputLow => (80, 120, 240, 255),
                ColorId::PinOutHigh => (240, 80, 80, 255),
                ColorId::PinOutLow => (80, 120, 240, 255),
                ColorId::ItemHovered => (80, 170, 255, 255),
                ColorId::TunnelColor1 => (100, 220, 100, 255),
                ColorId::TunnelColor2 => (255, 255, 250, 255),
                ColorId::TunnelColor3 => (210, 210, 230, 255),
                ColorId::TreeItemBg1 => (220, 235, 240, 255),
                ColorId::TreeItemBg2 => (220, 240, 235, 255),
                ColorId::TreeItemText1 => (50, 60, 80, 255),
                ColorId::TreeItemText2 => (75, 70, 10, 255),
                ColorId::ListItemBg1 => (240, 235, 245, 255),
                ColorId::ListItemBg2 => (255, 235, 155, 255),
                ColorId::ListItemText1 => (110, 95, 50, 255),
                ColorId::ListItemText2 => (110, 95, 50, 255),
                ColorId::PropLabelText => (8, 80, 65, 255),
                ColorId::PropHeaderText => (15, 110, 86, 255),
                ColorId::MemValText => (115, 35, 143, 255),
                ColorId::MemTypeText => (144, 64, 32, 255),
                ColorId::MemDataText => (32, 32, 144, 255),
                ColorId::MemDataBg => (255, 255, 252, 255),
                ColorId::TerminalRxBg => (35, 30, 60, 255),
                ColorId::TerminalRxText => (255, 255, 255, 255),
                ColorId::TerminalTxBg => (252, 252, 246, 255),
                ColorId::TerminalTxText => (0, 0, 0, 255),
                ColorId::MsgOkBg => (144, 238, 144, 255),
                ColorId::MsgOkText => (0, 0, 0, 255),
                ColorId::MsgWarnBg => (255, 165, 0, 255),
                ColorId::MsgWarnText => (255, 255, 255, 255),
                ColorId::MsgErrorBg => (255, 0, 0, 255),
                ColorId::MsgErrorText => (255, 255, 0, 255),
                ColorId::MsgDebugPausedBg => (0, 0, 255, 255),
                ColorId::MsgDebugPausedText => (255, 255, 255, 255),
                ColorId::MsgDebugRunningBg => (173, 216, 230, 255),
                ColorId::MsgDebugRunningText => (0, 0, 0, 255),
                ColorId::CoordsLabelText => (0, 0, 0, 255),
                ColorId::SimPauseTint => (217, 119, 6, 255),
                ColorId::ToggleButtonBg => (220, 220, 220, 200),
                ColorId::McuLabelBg => (228, 231, 236, 255),
                ColorId::McuLabelText => (29, 41, 57, 255),
                ColorId::McuLabelBorder => (208, 213, 221, 255),
                ColorId::McuPcVal1Text => (32, 32, 144, 255),
                ColorId::McuPcVal2Text => (48, 48, 184, 255),
                ColorId::ChipBody => (20, 30, 60, 255),
                ColorId::ChipLabelText => (160, 160, 180, 255),
                ColorId::EditorText => (30, 30, 30, 255),
                ColorId::EditorCurrentLine => (245, 242, 232, 255),
                ColorId::EditorDebugLine => (255, 210, 205, 255),
                ColorId::EditorGutterNum => (120, 125, 135, 255),
                ColorId::EditorGutterActiveNum => (20, 20, 25, 255),
                ColorId::EditorGutterBorder => (220, 220, 225, 255),
                ColorId::EditorSpace => (187, 187, 187, 255),
                ColorId::EditorFound => (210, 210, 255, 255),
                ColorId::EditorErrorUnderline => (240, 50, 50, 255),
                ColorId::EditorWarnUnderline => (255, 165, 0, 255),
                ColorId::EditorInfoUnderline => (90, 150, 255, 255),
                ColorId::EditorCodeBg => (240, 240, 244, 255),
                ColorId::EditorCodeBorder => (208, 208, 216, 255),
                ColorId::EditorCodeText => (0, 85, 170, 255),
                ColorId::LogStdout => (30, 30, 30, 255),
                ColorId::LogStderr => (200, 30, 30, 255),
                ColorId::LogErrorText => (220, 20, 20, 255),
                ColorId::LogWarnText => (190, 110, 0, 255),
                ColorId::LogInfoText => (20, 100, 200, 255),
                ColorId::LogSuccessText => (30, 140, 50, 255),
                ColorId::ScopeChannel1 => (255, 255, 0, 255),
                ColorId::ScopeChannel2 => (0, 255, 0, 255),
                ColorId::ScopeChannel3 => (0, 255, 255, 255),
                ColorId::ScopeChannel4 => (255, 0, 255, 255),
                ColorId::LaChannel1 => (231, 76, 60, 255),
                ColorId::LaChannel2 => (230, 126, 34, 255),
                ColorId::LaChannel3 => (241, 196, 15, 255),
                ColorId::LaChannel4 => (46, 204, 113, 255),
                ColorId::LaChannel5 => (26, 188, 156, 255),
                ColorId::LaChannel6 => (52, 152, 219, 255),
                ColorId::LaChannel7 => (155, 89, 182, 255),
                ColorId::LaChannel8 => (236, 240, 241, 255),
            }
        } else {
            match c {
                ColorId::ViewportBackground => (20, 20, 22, 255),
                ColorId::CanvasBackground => (35, 35, 35, 255),
                ColorId::CanvasGrid => (240, 240, 240, 80),
                ColorId::CanvasGridDots => (240, 240, 240, 45),
                ColorId::WireDefault => (40, 180, 100, 255),
                ColorId::WireHigh => (250, 100, 100, 255),
                ColorId::WireLow => (100, 100, 250, 255),
                ColorId::WireOpen => (120, 130, 150, 255),
                ColorId::WireInput => (100, 140, 250, 255),
                ColorId::ComponentBody => (60, 60, 60, 255),
                ColorId::ComponentText => (220, 220, 220, 255),
                ColorId::ComponentIdText => (100, 180, 255, 255),
                ColorId::ComponentValText => (255, 120, 120, 255),
                ColorId::ComponentBorder => (220, 220, 220, 255),
                ColorId::MeterDisplayText => (255, 255, 0, 255),
                ColorId::ShapeBody => (100, 100, 100, 255),
                ColorId::PinHigh => (250, 100, 100, 255),
                ColorId::PinLow => (100, 140, 250, 255),
                ColorId::PinOpenHigh => (80, 180, 80, 255),
                ColorId::PinOpenLow => (100, 140, 250, 255),
                ColorId::PinInputHigh => (250, 180, 100, 255),
                ColorId::PinInputLow => (100, 140, 250, 255),
                ColorId::PinOutHigh => (250, 100, 100, 255),
                ColorId::PinOutLow => (100, 140, 250, 255),
                ColorId::ItemHovered => (110, 190, 255, 255),
                ColorId::TunnelColor1 => (80, 180, 80, 255),
                ColorId::TunnelColor2 => (50, 50, 50, 255),
                ColorId::TunnelColor3 => (100, 100, 120, 255),
                ColorId::TreeItemBg1 => (45, 45, 45, 255),
                ColorId::TreeItemBg2 => (40, 40, 40, 255),
                ColorId::TreeItemText1 => (200, 210, 230, 255),
                ColorId::TreeItemText2 => (230, 220, 150, 255),
                ColorId::ListItemBg1 => (55, 50, 60, 255),
                ColorId::ListItemBg2 => (80, 60, 30, 255),
                ColorId::ListItemText1 => (220, 200, 150, 255),
                ColorId::ListItemText2 => (220, 200, 150, 255),
                ColorId::PropLabelText => (159, 225, 203, 255),
                ColorId::PropHeaderText => (93, 202, 165, 255),
                ColorId::MemValText => (176, 96, 208, 255),
                ColorId::MemTypeText => (208, 128, 80, 255),
                ColorId::MemDataText => (112, 112, 255, 255),
                ColorId::MemDataBg => (45, 45, 48, 255),
                ColorId::TerminalRxBg => (26, 24, 41, 255),
                ColorId::TerminalRxText => (255, 255, 255, 255),
                ColorId::TerminalTxBg => (30, 30, 30, 255),
                ColorId::TerminalTxText => (220, 220, 220, 255),
                ColorId::MsgOkBg => (46, 125, 50, 255),
                ColorId::MsgOkText => (255, 255, 255, 255),
                ColorId::MsgWarnBg => (191, 120, 0, 255),
                ColorId::MsgWarnText => (255, 255, 255, 255),
                ColorId::MsgErrorBg => (178, 34, 34, 255),
                ColorId::MsgErrorText => (255, 255, 0, 255),
                ColorId::MsgDebugPausedBg => (40, 60, 150, 255),
                ColorId::MsgDebugPausedText => (255, 255, 255, 255),
                ColorId::MsgDebugRunningBg => (70, 110, 150, 255),
                ColorId::MsgDebugRunningText => (255, 255, 255, 255),
                ColorId::CoordsLabelText => (255, 255, 255, 255),
                ColorId::SimPauseTint => (255, 179, 0, 255),
                ColorId::ToggleButtonBg => (40, 40, 40, 200),
                ColorId::McuLabelBg => (42, 45, 50, 255),
                ColorId::McuLabelText => (224, 230, 237, 255),
                ColorId::McuLabelBorder => (71, 84, 103, 255),
                ColorId::McuPcVal1Text => (100, 180, 245, 255),
                ColorId::McuPcVal2Text => (158, 202, 255, 255),
                ColorId::ChipBody => (20, 30, 60, 255),
                ColorId::ChipLabelText => (160, 160, 180, 255),
                ColorId::EditorText => (220, 220, 220, 255),
                ColorId::EditorCurrentLine => (45, 50, 60, 255),
                ColorId::EditorDebugLine => (90, 30, 30, 255),
                ColorId::EditorGutterNum => (140, 145, 155, 255),
                ColorId::EditorGutterActiveNum => (240, 245, 255, 255),
                ColorId::EditorGutterBorder => (55, 55, 60, 255),
                ColorId::EditorSpace => (60, 60, 60, 255),
                ColorId::EditorFound => (210, 210, 255, 255),
                ColorId::EditorErrorUnderline => (255, 80, 80, 255),
                ColorId::EditorWarnUnderline => (255, 180, 50, 255),
                ColorId::EditorInfoUnderline => (110, 170, 255, 255),
                ColorId::EditorCodeBg => (30, 30, 36, 255),
                ColorId::EditorCodeBorder => (58, 58, 68, 255),
                ColorId::EditorCodeText => (140, 220, 254, 255),
                ColorId::LogStdout => (220, 220, 220, 255),
                ColorId::LogStderr => (245, 100, 100, 255),
                ColorId::LogErrorText => (255, 90, 90, 255),
                ColorId::LogWarnText => (255, 180, 50, 255),
                ColorId::LogInfoText => (100, 180, 255, 255),
                ColorId::LogSuccessText => (80, 200, 120, 255),
                ColorId::ScopeChannel1 => (255, 255, 0, 255),
                ColorId::ScopeChannel2 => (0, 255, 0, 255),
                ColorId::ScopeChannel3 => (0, 255, 255, 255),
                ColorId::ScopeChannel4 => (255, 0, 255, 255),
                ColorId::LaChannel1 => (231, 76, 60, 255),
                ColorId::LaChannel2 => (230, 126, 34, 255),
                ColorId::LaChannel3 => (241, 196, 15, 255),
                ColorId::LaChannel4 => (46, 204, 113, 255),
                ColorId::LaChannel5 => (26, 188, 156, 255),
                ColorId::LaChannel6 => (52, 152, 219, 255),
                ColorId::LaChannel7 => (155, 89, 182, 255),
                ColorId::LaChannel8 => (236, 240, 241, 255),
            }
        }
    }

    /// Returns a Qt `#AARRGGBB` hex color string for full alpha or partial alpha.
    pub fn get_hex(c: ColorId, dark: bool) -> String {
        let (r, g, b, a) = Self::get_rgba(c, dark);
        if a == 255 {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            format!("#{a:02x}{r:02x}{g:02x}{b:02x}")
        }
    }

    /// 24-bit `0xRRGGBB` for syntax HTML, ignoring alpha.
    pub fn rgb_u32(c: ColorId, dark: bool) -> u32 {
        let (r, g, b, _) = Self::get_rgba(c, dark);
        u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
    }

    /// Returns the ColorId for a 0-indexed oscilloscope channel (0..3).
    pub fn scope_channel_color_id(ch: usize) -> ColorId {
        match ch {
            0 => ColorId::ScopeChannel1,
            1 => ColorId::ScopeChannel2,
            2 => ColorId::ScopeChannel3,
            _ => ColorId::ScopeChannel4,
        }
    }

    /// Returns the hex color string for a 0-indexed oscilloscope channel.
    pub fn scope_channel_hex(ch: usize, dark: bool) -> String {
        Self::get_hex(Self::scope_channel_color_id(ch), dark)
    }

    /// Returns the 4 oscilloscope channel colors as hex strings for the given theme mode.
    pub fn scope_colors(dark: bool) -> [String; 4] {
        [
            Self::get_hex(ColorId::ScopeChannel1, dark),
            Self::get_hex(ColorId::ScopeChannel2, dark),
            Self::get_hex(ColorId::ScopeChannel3, dark),
            Self::get_hex(ColorId::ScopeChannel4, dark),
        ]
    }

    /// Returns the ColorId for a 0-indexed logic analyzer channel (0..7).
    pub fn la_channel_color_id(ch: usize) -> ColorId {
        match ch {
            0 => ColorId::LaChannel1,
            1 => ColorId::LaChannel2,
            2 => ColorId::LaChannel3,
            3 => ColorId::LaChannel4,
            4 => ColorId::LaChannel5,
            5 => ColorId::LaChannel6,
            6 => ColorId::LaChannel7,
            _ => ColorId::LaChannel8,
        }
    }

    /// Returns the hex color string for a 0-indexed logic analyzer channel.
    pub fn la_channel_hex(ch: usize, dark: bool) -> String {
        Self::get_hex(Self::la_channel_color_id(ch), dark)
    }

    /// Returns the 8 logic analyzer channel colors as hex strings for the given theme mode.
    pub fn la_colors(dark: bool) -> [String; 8] {
        [
            Self::get_hex(ColorId::LaChannel1, dark),
            Self::get_hex(ColorId::LaChannel2, dark),
            Self::get_hex(ColorId::LaChannel3, dark),
            Self::get_hex(ColorId::LaChannel4, dark),
            Self::get_hex(ColorId::LaChannel5, dark),
            Self::get_hex(ColorId::LaChannel6, dark),
            Self::get_hex(ColorId::LaChannel7, dark),
            Self::get_hex(ColorId::LaChannel8, dark),
        ]
    }

    /// Returns `(lit_rgba, unlit_rgba)` for a named LED color.
    /// Parity with C++ LedBase / ColorTheme.
    pub fn led_color_rgba(name: &str) -> ((u8, u8, u8, u8), (u8, u8, u8, u8)) {
        match name.trim().to_ascii_lowercase().as_str() {
            "yellow" => ((255, 238, 34, 255), (68, 68, 17, 255)),
            "red" => ((255, 34, 34, 255), (68, 17, 17, 255)),
            "green" => ((34, 221, 68, 255), (17, 68, 17, 255)),
            "blue" | "blue_super" => ((34, 136, 255, 255), (17, 34, 68, 255)),
            "orange" => ((255, 136, 0, 255), (68, 34, 0, 255)),
            "purple" | "uv" => ((170, 68, 255, 255), (51, 17, 51, 255)),
            "white" => ((255, 255, 255, 255), (68, 68, 68, 255)),
            "infrared" => ((255, 68, 102, 255), (68, 17, 26, 255)),
            _ => ((255, 238, 34, 255), (68, 68, 17, 255)),
        }
    }

    /// Returns `(lit_hex, unlit_hex)` formatted as `#rrggbb` for a named LED color.
    pub fn led_color_hex(name: &str) -> (String, String) {
        let (lit, unlit) = Self::led_color_rgba(name);
        (
            format!("#{:02x}{:02x}{:02x}", lit.0, lit.1, lit.2),
            format!("#{:02x}{:02x}{:02x}", unlit.0, unlit.1, unlit.2),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_hovered_selection_colors() {
        // Light theme should be bright blue
        let light_hover = ColorTheme::get_rgba(ColorId::ItemHovered, false);
        assert_eq!(light_hover, (80, 170, 255, 255));
        assert_eq!(ColorTheme::get_hex(ColorId::ItemHovered, false), "#50aaff");

        // Dark theme should be high-contrast vibrant blue
        let dark_hover = ColorTheme::get_rgba(ColorId::ItemHovered, true);
        assert_eq!(dark_hover, (110, 190, 255, 255));
        assert_eq!(ColorTheme::get_hex(ColorId::ItemHovered, true), "#6ebeff");
    }

    #[test]
    fn editor_colors_match_cpp_code_editor() {
        assert_eq!(
            ColorTheme::get_rgba(ColorId::EditorText, false),
            (30, 30, 30, 255)
        );
        assert_eq!(
            ColorTheme::get_rgba(ColorId::EditorText, true),
            (220, 220, 220, 255)
        );
        assert_eq!(
            ColorTheme::get_rgba(ColorId::EditorCurrentLine, false),
            (245, 242, 232, 255)
        );
        assert_eq!(
            ColorTheme::get_rgba(ColorId::EditorCurrentLine, true),
            (45, 50, 60, 255)
        );
        assert_eq!(
            ColorTheme::get_rgba(ColorId::EditorDebugLine, false),
            (255, 210, 205, 255)
        );
        assert_eq!(
            ColorTheme::get_rgba(ColorId::EditorDebugLine, true),
            (90, 30, 30, 255)
        );
        assert_eq!(ColorTheme::rgb_u32(ColorId::EditorText, true), 0xDCDCDC);
        assert_eq!(ColorTheme::rgb_u32(ColorId::EditorSpace, false), 0xBBBBBB);
    }

    #[test]
    fn test_led_colors_parity() {
        let (lit_red, unlit_red) = ColorTheme::led_color_hex("Red");
        assert_eq!(lit_red, "#ff2222");
        assert_eq!(unlit_red, "#441111");

        let (lit_green, unlit_green) = ColorTheme::led_color_hex("Green");
        assert_eq!(lit_green, "#22dd44");
        assert_eq!(unlit_green, "#114411");

        let (lit_yellow, unlit_yellow) = ColorTheme::led_color_hex("Yellow");
        assert_eq!(lit_yellow, "#ffee22");
        assert_eq!(unlit_yellow, "#444411");

        let (lit_blue, unlit_blue) = ColorTheme::led_color_hex("Blue");
        assert_eq!(lit_blue, "#2288ff");
        assert_eq!(unlit_blue, "#112244");

        // Case insensitivity and fallback
        let (lit_case, unlit_case) = ColorTheme::led_color_hex("red");
        assert_eq!(lit_case, "#ff2222");
        assert_eq!(unlit_case, "#441111");

        let (lit_fallback, _) = ColorTheme::led_color_hex("unknown");
        assert_eq!(lit_fallback, "#ffee22");
    }
}
