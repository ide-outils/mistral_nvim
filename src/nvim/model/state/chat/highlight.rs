use std::sync::LazyLock;

use nvim_oxi::api;

// Namespace dédié pour les highlights de statusline
#[allow(dead_code)]
static NS: LazyLock<u32> = LazyLock::new(|| api::create_namespace("mistral_statusline"));

// Définition des groupes de highlight avec des couleurs optimisées pour la lisibilité
pub(super) const NAME: &'static str = "MistralBarStatus";
pub(super) static HL_NAME: LazyLock<&str> = LazyLock::new(|| {
    // let opts = api::opts::SetHighlightOpts::builder()
    //     .background("#1e1e2e")
    //     .foreground("#89b4fa")
    //     .bold(true)
    //     .build();
    // api::set_hl(*NS, hl, &opts).unwrap();
    api::command(&format!(
        "highlight {} guifg=#89b4fa guibg=#1e1e2e guisp=#89b4fa ctermfg=75 ctermbg=235 cterm=bold",
        NAME
    ))
    .unwrap();
    NAME
});

pub(super) const PAGE: &'static str = "MistralBarPage";
pub(super) static HL_PAGE: LazyLock<&str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#f9e2af guibg=#1e1e2e guisp=#f9e2af ctermfg=223 ctermbg=235 cterm=bold",
        PAGE
    ))
    .unwrap_or(());
    PAGE
});

pub(super) const MODEL: &'static str = "MistralBarModel";
pub(super) static HL_MODEL: LazyLock<&str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#a6e3a1 guibg=#1e1e2e guisp=#a6e3a1 ctermfg=119 ctermbg=235 cterm=bold",
        MODEL
    ))
    .unwrap_or(());
    MODEL
});

pub(super) const USAGE: &'static str = "MistralBarUsage";
pub(super) static HL_USAGE: LazyLock<&str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#f38ba8 guibg=#1e1e2e guisp=#f38ba8 ctermfg=204 ctermbg=235 cterm=bold",
        USAGE
    ))
    .unwrap_or(());
    USAGE
});

pub(super) const MODE: &'static str = "MistralBarMode";
pub(super) static HL_MODE: LazyLock<&str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#cba6f7 guibg=#1e1e2e guisp=#cba6f7 ctermfg=141 ctermbg=235 cterm=bold",
        MODE
    ))
    .unwrap_or(());
    MODE
});

// pub(super) const ERROR: &'static str = "MistralBarError";
// pub(super) static HL_ERROR: LazyLock<&str> = LazyLock::new(|| {
//     api::command(&format!(
//         "highlight {} guifg=#f38ba8 guibg=#1e1e2e guisp=#f38ba8 ctermfg=204 ctermbg=235 cterm=bold,underline",
//         ERROR
//     ))
//     .unwrap_or(());
//     ERROR
// });

pub(super) const ROLE_USER: &'static str = "MistralRoleUser";
pub(super) static HL_ROLE_USER: LazyLock<&'static str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#89b4fa guibg=#1e1e2e guisp=#89b4fa ctermfg=75 ctermbg=235 cterm=bold",
        ROLE_USER
    ))
    .unwrap_or(());
    ROLE_USER
});

pub(super) const ROLE_SYSTEM: &'static str = "MistralRoleSystem";
pub(super) static HL_ROLE_SYSTEM: LazyLock<&'static str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#f9e2af guibg=#1e1e2e guisp=#f9e2af ctermfg=223 ctermbg=235 cterm=bold",
        ROLE_SYSTEM
    ))
    .unwrap_or(());
    ROLE_SYSTEM
});

pub(super) const ROLE_ASSISTANT: &'static str = "MistralRoleAssistant";
pub(super) static HL_ROLE_ASSISTANT: LazyLock<&'static str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#a6e3a1 guibg=#1e1e2e guisp=#a6e3a1 ctermfg=119 ctermbg=235 cterm=bold",
        ROLE_ASSISTANT
    ))
    .unwrap_or(());
    ROLE_ASSISTANT
});

pub(super) const ROLE_TOOL: &'static str = "MistralRoleTool";
pub(super) static HL_ROLE_TOOL: LazyLock<&'static str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#f38ba8 guibg=#1e1e2e guisp=#f38ba8 ctermfg=204 ctermbg=235 cterm=bold",
        ROLE_TOOL
    ))
    .unwrap_or(());
    ROLE_TOOL
});

pub(super) const STATUS_COMPLETED: &'static str = "MistralStatusCompleted";
pub(super) static HL_STATUS_COMPLETED: LazyLock<&'static str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#a6e3a1 guibg=#1e1e2e guisp=#a6e3a1 ctermfg=119 ctermbg=235 cterm=bold",
        STATUS_COMPLETED
    ))
    .unwrap_or(());
    STATUS_COMPLETED
});

pub(super) const STATUS_PARTIAL: &'static str = "MistralStatusPartial";
pub(super) static HL_STATUS_PARTIAL: LazyLock<&'static str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#f9e2af guibg=#1e1e2e guisp=#f9e2af ctermfg=223 ctermbg=235 cterm=bold",
        STATUS_PARTIAL
    ))
    .unwrap_or(());
    STATUS_PARTIAL
});

pub(super) const STATUS_FAILED: &'static str = "MistralStatusFailed";
pub(super) static HL_STATUS_FAILED: LazyLock<&'static str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#f38ba8 guibg=#1e1e2e guisp=#f38ba8 ctermfg=204 ctermbg=235 cterm=bold,underline",
        STATUS_FAILED
    ))
    .unwrap_or(());
    STATUS_FAILED
});

pub(super) const STATUS_CREATED: &'static str = "MistralStatusCreated";
pub(super) static HL_STATUS_CREATED: LazyLock<&'static str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#89b4fa guibg=#1e1e2e guisp=#89b4fa ctermfg=75 ctermbg=235 cterm=bold",
        STATUS_CREATED
    ))
    .unwrap_or(());
    STATUS_CREATED
});

pub(super) const STATUS_INITIALISED: &'static str = "MistralStatusInitialised";
pub(super) static HL_STATUS_INITIALISED: LazyLock<&'static str> = LazyLock::new(|| {
    api::command(&format!(
        "highlight {} guifg=#cba6f7 guibg=#1e1e2e guisp=#cba6f7 ctermfg=141 ctermbg=235 cterm=bold",
        STATUS_INITIALISED
    ))
    .unwrap_or(());
    STATUS_INITIALISED
});
