/// Centralized UI strings for internationalization
/// All user-facing text should be defined here

pub const APP_TITLE: &str = "NTerm - Terminal Emulator";
pub const APP_ID: &str = "com.nterm.Terminal";

// Tab labels
pub const TAB_LABEL_TERMINAL: &str = "Terminal";

// Tooltips
pub const TOOLTIP_SEARCH: &str = "Search in terminal (Ctrl+Shift+F)";
pub const TOOLTIP_SEARCH_PREVIOUS: &str = "Previous result (Shift+Enter)";
pub const TOOLTIP_SEARCH_NEXT: &str = "Next result (Enter)";

// Dialog messages
pub const DIALOG_CLOSE_TAB_TITLE: &str = "Close tab?";
pub const DIALOG_CLOSE_TAB_MESSAGE: &str =
    "The program '{}' is running in this tab. Do you really want to close it?";
pub const BUTTON_CANCEL: &str = "Cancel";
pub const BUTTON_CLOSE: &str = "Close";

// Error messages
pub const ERROR_FAILED_TO_OPEN_URL: &str = "Failed to open URL {}: {}";
pub const ERROR_LAUNCHING_SHELL: &str = "Error launching shell: {}";
pub const ERROR_COMPILE_HTTP_REGEX: &str = "Failed to compile HTTP/FTP regex pattern";
pub const ERROR_COMPILE_EMAIL_REGEX: &str = "Failed to compile email regex pattern";

/// Format a string with a single parameter
pub fn format_single(template: &str, param: &str) -> String {
    template.replace("{}", param)
}

/// Format the close tab dialog message with program name
pub fn format_close_tab_message(program_name: &str) -> String {
    DIALOG_CLOSE_TAB_MESSAGE.replace("{}", program_name)
}
