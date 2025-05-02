use anyhow::{bail, Context, Result};
use git::Git;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::{read_dir, read_to_string},
    path::Path,
};

mod git;

const REPOSITORY_PATH: &str = "tree-sitter-grammar";
const PATCH_BRANCH: &str = "zed-hyprlang-extension-patches";
const TEMPORARY_BRANCH: &str = "zed-hyprlang-extension-temp";

pub const DEBUG: Option<&str> = option_env!("DEBUG");

#[derive(Deserialize)]
struct Extension {
    pub grammars: HashMap<String, Grammar>,
}

#[derive(Deserialize, Debug)]
struct Grammar {
    pub repository: String,
    pub commit: String,
}

fn main() {
    // rerun the build script when the extension.toml file changes since this could indicate
    // a grammar version change
    println!("cargo::rerun-if-changed=extension.toml");

    let out_dir = std::env::var("OUT_DIR").unwrap_or(String::from("build"));
    let manifest_env = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let manifest_dir = Path::new(&manifest_env);

    println!(
        "cargo::warning=using manifest dir {}",
        manifest_dir.to_str().unwrap_or_default()
    );

    if DEBUG.is_some() {
        println!("cargo::warning=using output dir {out_dir}");
    }

    let extension = match read_to_string(manifest_dir.join("extension.toml")) {
        Ok(extension_str) => match toml::from_str::<Extension>(&extension_str) {
            Ok(extension) => extension,
            Err(err) => return println!("cargo::error=unable to parse extension.toml: {err}"),
        },
        Err(err) => return println!("cargo::error=unable to read extension.toml: {err}"),
    };

    let hyprlang_grammar = match extension.grammars.get("hyprlang") {
        Some(grammar) => grammar,
        None => return println!("cargo::error=extension does not specify a hyprlang grammar"),
    };

    let repository = Git::new(Path::new(&out_dir).join(REPOSITORY_PATH));

    if let Err(err) = checkout_repository(
        &repository,
        &hyprlang_grammar.repository,
        &hyprlang_grammar.commit,
    ) {
        return println!(
            "cargo::error=unable to checkout hyprlang grammar {} [{}]: {}",
            err.chain()
                .map(|err| err.to_string())
                .collect::<Vec<String>>()
                .join(": "),
            hyprlang_grammar.repository,
            hyprlang_grammar.commit
        );
    }

    let patches_dir = manifest_dir.join("patches");
    let patches = match read_dir(&patches_dir) {
        Ok(dir) => dir,
        Err(err) => {
            return println!(
                "cargo::error=unable to read patches from {}: {err}",
                patches_dir.to_string_lossy()
            )
        }
    };

    for patch in patches.flatten() {
        if let Some(path) = patch.path().to_str() {
            if let Err(err) = repository.apply(path) {
                return println!("cargo::error=unable to apply patch from {path}: {err}");
            };
        }
    }

    let queries = match read_dir(repository.directory().join("queries/hyprlang")) {
        Ok(dir) => dir,
        Err(err) => {
            return println!("cargo::error=unable to read hyprlang grammar directory: {err}")
        }
    };

    let languages = match read_dir(manifest_dir.join("languages")) {
        Ok(dir) => dir
            .into_iter()
            .filter_map(|lang| lang.ok().map(|lang| lang.path()))
            .collect::<Vec<_>>(),
        Err(err) => return println!("cargo:error=unable to read languages dir: {err}"),
    };

    for query in queries.flatten() {
        for language in &languages {
            if let Err(err) = std::fs::copy(query.path(), language.join(query.file_name())) {
                println!(
                    "cargo::warning=unable to copy {} to {}: {err}",
                    query.path().to_string_lossy(),
                    language.join(query.file_name()).to_string_lossy()
                )
            }
        }
    }
}

fn checkout_repository(repository: &Git, url: &str, rev: &str) -> Result<()> {
    if repository.directory().exists() {
        let remotes = repository.remotes().context("failed to get git remotes")?;
        if !remotes
            .iter()
            .any(|(name, remote_url)| name == "origin" && remote_url == url)
        {
            bail!(
                "grammar directory {} already exists, but is not a git clone of {}",
                repository.directory().display(),
                url
            );
        }

        if !repository.has_branch(TEMPORARY_BRANCH) {
            repository
                .checkout_branch(TEMPORARY_BRANCH, true, None)
                .context("unable to checkout temporary branch")?;
        }

        if repository.has_branch(PATCH_BRANCH) {
            repository.reset(true).context("unable to hard reset working tree")?;
            repository.clean().context("unable to clean working tree")?;
            repository
                .checkout_branch(TEMPORARY_BRANCH, false, None)
                .context("unable to checkout temporary branch")?;
            repository
                .delete_branch(PATCH_BRANCH)
                .context("unable to delete branch")?
        }
    } else {
        repository
            .init()
            .context("unable to initialize new repository")?;
        repository
            .add_remote("origin", url)
            .context("unable to add git remote")?;
        repository
            .checkout_branch(TEMPORARY_BRANCH, true, None)
            .context("unable to create and checkout temporary branch")?;
    }

    repository
        .fetch("origin", Some(rev))
        .context("unable to fetch git revision")?;
    repository
        .checkout_branch(PATCH_BRANCH, true, Some(rev))
        .context("unable to checkout git patch branch")?;

    Ok(())
}
