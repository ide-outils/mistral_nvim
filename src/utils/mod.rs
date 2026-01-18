use nvim_oxi::api::{self, types};

use crate::notify::NotifyExt as _;

pub mod logger;
pub mod notify;
pub mod tool_id;

pub fn set_option<Opt>(buffer: &api::Buffer, var: &str, value: Opt)
where
    Opt: nvim_oxi::conversion::ToObject,
{
    let buffer = buffer.clone();
    let opt_buffer = api::opts::OptionOpts::builder().buffer(buffer).build();
    api::set_option_value(var, value, &opt_buffer).notify_error();
}

pub fn set_option_win<Opt>(win: &api::Window, var: &str, value: Opt)
where
    Opt: nvim_oxi::conversion::ToObject,
{
    let win = win.clone();
    let opt_buffer = api::opts::OptionOpts::builder().win(win).build();
    api::set_option_value(var, value, &opt_buffer).notify_error();
}

pub fn get_option<Opt>(buffer: &api::Buffer, var: &str) -> std::result::Result<Opt, api::Error>
where
    Opt: nvim_oxi::conversion::FromObject,
{
    let buffer = buffer.clone();
    let opt_buffer = api::opts::OptionOpts::builder().buffer(buffer).build();
    api::get_option_value::<Opt>(var, &opt_buffer)
}

pub fn get_option_win<Opt>(win: &api::Window, var: &str) -> std::result::Result<Opt, api::Error>
where
    Opt: nvim_oxi::conversion::FromObject,
{
    let win = win.clone();
    let opt_buffer = api::opts::OptionOpts::builder().win(win).build();
    api::get_option_value::<Opt>(var, &opt_buffer)
}

/// Helper to set keymap more readability
#[macro_export]
macro_rules! set_keymaps {
    {$(
        // the shortcuts and its modes for the next ones
        $shortcut:ident ($($mode:ident),+):
        // the keys           the method           the clones and its type
        $($binding:literal => {$method:expr} <= <$($clone:ident: $clone_type:ident),*>)+
    )+}
    => {
        $(
            $shortcut.set_modes(vec![$($mode),+]);
            $(
            {
                $(
                    let $clone = $clone_type::clone(&$clone);
                )*

                $shortcut
                    .callback(move |_| $method)
                    .set_keymap($binding);
                    // .set_keymap($binding, move |_| $method);
            }
            )+
        )+
    }
}

pub struct ShortcutBuilder {
    opts: api::opts::SetKeymapOptsBuilder,
    modes: std::collections::HashSet<types::Mode>,
    buffer: api::Buffer,
}

impl ShortcutBuilder {
    pub fn new(buffer: api::Buffer) -> Self {
        let mut opts = api::opts::SetKeymapOpts::builder();
        opts.nowait(true).noremap(true).silent(true);
        let modes = Default::default();
        Self { opts, modes, buffer }
    }
    pub fn callback<'builder, F>(&'builder mut self, callback: F) -> &'builder mut Self
    where
        F: FnMut(()) + 'static,
    {
        self.opts.callback(callback);
        self
    }
    pub fn clear_modes<'builder>(&'builder mut self) -> &'builder mut Self {
        self.modes.clear();
        self
    }
    pub fn set_modes<'builder>(&'builder mut self, modes: impl IntoIterator<Item = types::Mode>) -> &'builder mut Self {
        self.modes.clear();
        self.modes = modes.into_iter().collect();
        self
    }
    pub fn normal<'builder>(&'builder mut self) -> &'builder mut Self {
        self.modes.insert(types::Mode::Normal);
        self
    }
    pub fn insert<'builder>(&'builder mut self) -> &'builder mut Self {
        self.modes.insert(types::Mode::Insert);
        self
    }
    pub fn visual<'builder>(&'builder mut self) -> &'builder mut Self {
        self.modes.insert(types::Mode::Visual);
        self
    }
    pub fn visual_select<'builder>(&'builder mut self) -> &'builder mut Self {
        self.modes.insert(types::Mode::VisualSelect);
        self
    }
    pub fn set_keymap<'builder>(&'builder mut self, shortcut: &str) -> &'builder mut Self {
        let Self { modes, buffer, opts } = self;
        let opts = opts.build();
        for mode in modes.iter().cloned() {
            // Need to clone opts, sometimes the command is not set properly with callback.
            if let Err(error) = buffer.set_keymap(mode, shortcut, "", &opts.clone()) {
                crate::notify::error(&format!("Can't set_keymap : {error}"));
            }
        }
        self
    }
}
