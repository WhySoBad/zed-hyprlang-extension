use std::fs;

use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result};

/// Name of the linux binary release asset
const HYPRLS_LINUX_BINARY_NAME: &str = "hyprls";
/// Name of the macos binary release asset
const HYPRLS_MACOS_BINARY_NAME: &str = "hyprls";
/// Name of the windows binary release asset
const HYPRLS_WINDOWS_BINARY_NAME: &str = "hyprls";

struct HyprlangExtension {
    cached_binary_path: Option<String>,
}

impl HyprlangExtension {
    fn make_language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let lsp_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree).ok();
        let binary_settings = lsp_settings
            .as_ref()
            .and_then(|lsp_settings| lsp_settings.binary.as_ref());

        let args = binary_settings
            .and_then(|binary_settings| binary_settings.arguments.as_ref())
            .cloned()
            .unwrap_or_default();

        let env = binary_settings
            .and_then(|binary_settings| binary_settings.env.as_ref())
            .map(|env| {
                env.iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // if a path is specified, always try to use the binary at this path instead of the one from the github release
        if let Some(path) = binary_settings.and_then(|binary_settings| binary_settings.path.clone())
        {
            return Ok(zed::Command {
                command: path,
                args,
                env,
            });
        }

        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).is_ok_and(|stat| stat.is_file()) {
                return Ok(zed::Command {
                    command: path.clone(),
                    args,
                    env,
                });
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed_extension_api::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::latest_github_release(
            "hyprland-community/hyprls",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;
        let version = release
            .version
            .strip_prefix("v")
            .unwrap_or(&release.version);
        let version_dir = format!("hyprlang-{version}");

        let binary_asset_name = match zed::current_platform() {
            (zed::Os::Linux, _) => HYPRLS_LINUX_BINARY_NAME,
            (zed::Os::Mac, _) => HYPRLS_MACOS_BINARY_NAME,
            (zed::Os::Windows, _) => HYPRLS_WINDOWS_BINARY_NAME,
        };

        let binary_path = format!("{version_dir}/{binary_asset_name}");

        let hyprls_asset = release
            .assets
            .iter()
            .find(|asset| asset.name == binary_asset_name)
            .ok_or_else(|| format!("asset not found in github release: {binary_asset_name}"))?;

        fs::create_dir_all(&version_dir)
            .map_err(|err| format!("failed to create directory '{version_dir}': {err}"))?;

        // if we have already downloaded a binary for the current version, we don't need to download it again
        if !fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed_extension_api::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &hyprls_asset.download_url,
                &version_dir,
                zed_extension_api::DownloadedFileType::Zip,
            )
            .map_err(|err| format!("failed to download file: {err}"))?;

            let entries = fs::read_dir(".")
                .map_err(|err| format!("failed to list current working directory: {err}"))?;

            for entry in entries {
                let entry =
                    entry.map_err(|err| format!("failed to workspace subdirectory: {err}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        self.cached_binary_path = Some(binary_path.clone());

        Ok(zed::Command {
            command: binary_path,
            args,
            env,
        })
    }
}

impl zed::Extension for HyprlangExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        self.make_language_server_command(language_server_id, worktree)
    }

    fn language_server_initialization_options(
        &mut self,
        server_id: &LanguageServerId,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<Option<zed_extension_api::serde_json::Value>> {
        let settings = LspSettings::for_worktree(server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.initialization_options.clone())
            .unwrap_or_default();
        Ok(Some(settings))
    }

    fn language_server_workspace_configuration(
        &mut self,
        server_id: &LanguageServerId,
        worktree: &zed_extension_api::Worktree,
    ) -> Result<Option<zed_extension_api::serde_json::Value>> {
        let settings = LspSettings::for_worktree(server_id.as_ref(), worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings.clone())
            .unwrap_or_default();
        Ok(Some(settings))
    }
}

zed::register_extension!(HyprlangExtension);
