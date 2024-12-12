use zed_extension_api as zed;

struct HyprlangExtension;

impl zed::Extension for HyprlangExtension {
    fn new() -> Self where Self: Sized {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let path = worktree
            .which("hyprls")
            .ok_or(String::from("hyprls must be installed manually and has to be available in $PATH"))?;

        let arguments = zed::settings::LspSettings::for_worktree("hyprls", worktree)
            .map(|settings| {
                settings.binary.and_then(|binary| binary.arguments).unwrap_or_default()
            })
            .unwrap_or_default();

        Ok(zed::Command {
            command: path,
            args: arguments,
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(HyprlangExtension);