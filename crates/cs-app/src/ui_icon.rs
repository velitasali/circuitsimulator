//! Canonical inventory of Material Symbols Rounded UI icons used across Rust and QML.
//!
//! Keeping every icon identifier in [`UiIcon`] ensures Rust call sites stay
//! type-checked and lets tests verify that every QML icon reference and
//! exported SVG asset stays in sync.

macro_rules! define_ui_icons {
    ($($variant:ident => $name:literal),* $(,)?) => {
        /// Every UI icon referenced by Rust models/menus or QML components.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[allow(dead_code)]
        pub enum UiIcon {
            $($variant),*
        }

        impl UiIcon {
            /// Slice of every defined [`UiIcon`] variant.
            #[allow(dead_code)]
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// Canonical icon identifier (Material Symbols Rounded ligature and SVG stem).
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $name),*
                }
            }

            /// Rust enum variant name (`PascalCase`).
            #[allow(dead_code)]
            pub const fn variant_name(self) -> &'static str {
                match self {
                    $(Self::$variant => stringify!($variant)),*
                }
            }

            /// Resolves a string identifier to its [`UiIcon`] variant, if registered.
            #[allow(dead_code)]
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

define_ui_icons! {
    AccountTree => "account_tree",
    Add => "add",
    ArrowDropDown => "arrow_drop_down",
    ArrowDropUp => "arrow_drop_up",
    Article => "article",
    BugReport => "bug_report",
    Build => "build",
    Check => "check",
    ChevronLeft => "chevron_left",
    ChevronRight => "chevron_right",
    Circle => "circle",
    Close => "close",
    Code => "code",
    ContentCopy => "content_copy",
    ContentCut => "content_cut",
    ContentPaste => "content_paste",
    CreateNewFolder => "create_new_folder",
    CropFree => "crop_free",
    DataObject => "data_object",
    Delete => "delete",
    Description => "description",
    DeveloperBoard => "developer_board",
    DockToLeft => "dock_to_left",
    Draft => "draft",
    DragIndicator => "drag_indicator",
    Edit => "edit",
    ExitToApp => "exit_to_app",
    ExpandLess => "expand_less",
    ExpandMore => "expand_more",
    FastForward => "fast_forward",
    FileDownload => "file_download",
    FileOpen => "file_open",
    FitScreen => "fit_screen",
    Flip => "flip",
    Folder => "folder",
    FolderOff => "folder_off",
    FolderOpen => "folder_open",
    FormatAlignLeft => "format_align_left",
    Grid4x4 => "grid_4x4",
    GridOn => "grid_on",
    GridView => "grid_view",
    Help => "help",
    Image => "image",
    Info => "info",
    Insights => "insights",
    InvertColors => "invert_colors",
    IosShare => "ios_share",
    Keyboard => "keyboard",
    KeyboardReturn => "keyboard_return",
    LibraryBooks => "library_books",
    Lightbulb => "lightbulb",
    Link => "link",
    LinkOff => "link_off",
    List => "list",
    LocalLibrary => "local_library",
    Memory => "memory",
    Menu => "menu",
    Monitoring => "monitoring",
    MotionPhotosOn => "motion_photos_on",
    NoteAdd => "note_add",
    OpenInNew => "open_in_new",
    Pause => "pause",
    PauseCircle => "pause_circle",
    PestControl => "pest_control",
    PlayArrow => "play_arrow",
    PlayCircle => "play_circle",
    PowerSettingsNew => "power_settings_new",
    Redo => "redo",
    Refresh => "refresh",
    RestartAlt => "restart_alt",
    RotateLeft => "rotate_left",
    RotateRight => "rotate_right",
    Save => "save",
    SaveAs => "save_as",
    Schema => "schema",
    Search => "search",
    SelectAll => "select_all",
    Settings => "settings",
    SettingsApplications => "settings_applications",
    ShowChart => "show_chart",
    SkipNext => "skip_next",
    StepInto => "step_into",
    StepOver => "step_over",
    Stop => "stop",
    StopCircle => "stop_circle",
    SwapVert => "swap_vert",
    SyncAlt => "sync_alt",
    TabCloseAll => "tab_close_all",
    TabCloseLeft => "tab_close_left",
    TabCloseOthers => "tab_close_others",
    TabCloseRight => "tab_close_right",
    TableChart => "table_chart",
    Terminal => "terminal",
    Tune => "tune",
    Undo => "undo",
    Upload => "upload",
    Visibility => "visibility",
    VisibilityOff => "visibility_off",
    Warning => "warning",
    Waves => "waves",
    ZoomIn => "zoom_in",
    ZoomOut => "zoom_out",
    ZoomOutMap => "zoom_out_map",
}
