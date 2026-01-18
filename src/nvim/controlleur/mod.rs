use crate::{
    n,
    nvim::{
        controlleur::fim::{ncmd, nmap, vcmd, vmap},
        model::{Locker as _, SharedState},
    },
    v,
};

pub mod chat;
mod fim;
mod form;

// pub fn setup(sender: mpsc::UnboundedSender<NvimEnveloppe>, state: SharedState) -> crate::Result<()> {
pub fn setup(s: &SharedState) -> crate::Result<()> {
    use nvim_oxi::api::opts;

    let c_opts = opts::CreateCommandOpts::builder;
    let k_opts = opts::SetKeymapOpts::builder;

    use crate::messages::NvimMessage::*;

    // Fill In the Middle
    let d = "Applique FIM sur la sélection visuelle.";
    vmap(s, v!(FimVisual), "<Leader>mf", k_opts().desc(d).noremap(true))?;
    vcmd(s, v!(FimVisual), "MistralFIMVisual", c_opts().desc(d))?;

    let d = "Applique FIM sur la fonction sous le curseur.";
    nmap(s, n!(FimFunction), "<Leader>mff", k_opts().desc(d).noremap(true))?;
    ncmd(s, n!(FimFunction), "MistralFIMFunction", c_opts().desc(d))?;

    let d = "Applique FIM en utilisant l'intégralité du fichier avec la position du curseur.";
    nmap(s, n!(FimCursorLine), "<Leader>mfc", k_opts().desc(d).noremap(true))?;
    ncmd(s, n!(FimCursorLine), "MistralFIMCursor", c_opts().desc(d))?;

    chat::setup_commands(s)?;

    #[cfg(not(feature = "no_logs"))]
    {
        use nvim_oxi::api::{create_user_command as cmd, opts::CreateCommandOpts, types::CommandNArgs};
        let d = "Change Log Level.";
        let one = CommandNArgs::One;
        let opts = CreateCommandOpts::builder().desc(d).nargs(one).build();
        cmd(
            "MistralLogLevel",
            move |args| {
                crate::utils::logger::THRESHOLD
                    .write()
                    .unwrap()
                    .set_by_args(args)
            },
            &opts,
        )?;
    }

    Ok(())
}
