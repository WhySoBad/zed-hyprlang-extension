use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs::{create_dir_all, read_dir, read_to_string},
    path::{Path, PathBuf},
    process::Command,
};
use toml;

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

    if let Err(err) = checkout_hyprlang_grammar(
        manifest_dir.join(out_dir).join("hyprlang"),
        &hyprlang_grammar.repository,
        &hyprlang_grammar.commit,
    ) {
        println!(
            "cargo::error=unable to checkout hyprlang grammar {} [{}]: {err}",
            hyprlang_grammar.repository, hyprlang_grammar.commit
        )
    }

    let queries = match read_dir(manifest_dir.join("grammars/hyprlang").join("queries/hyprlang")) {
        Ok(dir) => dir,
        Err(err) => return println!("cargo::error=unable to read hyprlang grammar dir: {err}"),
    };

    let languages = match read_dir(manifest_dir.join("languages")) {
        Ok(dir) => {
            dir.into_iter().filter_map(|lang| lang.ok().map(|lang| lang.path())).collect::<Vec<_>>()
        },
        Err(err) => return println!("cargo:error=unable to read languages dir: {err}"),
    };

    let shared_queries = match read_dir(manifest_dir.join("hyprlang")) {
        Ok(dir) => dir,
        Err(err) => return println!("cargo::error=unable to read shared treesitter queries directory: {err}"),
    };

    for query in queries.chain(shared_queries) {
        if let Ok(query) = query {
            for language in &languages {
                if let Err(err) = std::fs::copy(query.path(), language.join(query.file_name())) {
                    println!("cargo::warning=unable to copy {} to {}: {err}", query.path().to_string_lossy(), language.join(query.file_name()).to_string_lossy())
                }
            }
        }
    }
}

// see: https://github.com/zed-industries/zed/blob/56f13ddc50af5713e270027a9dd71dc7b00203c2/crates/extension/src/extension_builder.rs#L263
fn checkout_hyprlang_grammar(directory: PathBuf, url: &str, rev: &str) -> Result<()> {
    let git_dir = &directory.join(".git");

    if directory.exists() {
        let remotes_output = Command::new("git")
            .arg("--git-dir")
            .arg(&git_dir)
            .args(["remote", "-v"])
            .output()?;
        let has_remote = remotes_output.status.success()
            && String::from_utf8_lossy(&remotes_output.stdout)
                .lines()
                .any(|line| {
                    let mut parts = line.split(|c: char| c.is_whitespace());
                    parts.next() == Some("origin") && parts.any(|part| part == url)
                });
        if !has_remote {
            bail!(
                "grammar directory '{}' already exists, but is not a git clone of '{}'",
                directory.display(),
                url
            );
        }
    } else {
        create_dir_all(&directory).with_context(|| {
            format!("failed to create grammar directory {}", directory.display(),)
        })?;
        let init_output = Command::new("git")
            .arg("init")
            .current_dir(&directory)
            .output()?;
        if !init_output.status.success() {
            bail!(
                "failed to run `git init` in directory '{}'",
                directory.display()
            );
        }

        let remote_add_output = Command::new("git")
            .arg("--git-dir")
            .arg(&git_dir)
            .args(["remote", "add", "origin", url])
            .output()
            .context("failed to execute `git remote add`")?;
        if !remote_add_output.status.success() {
            bail!(
                "failed to add remote {url} for git repository {}",
                git_dir.display()
            );
        }
    }

    let fetch_output = Command::new("git")
        .arg("--git-dir")
        .arg(&git_dir)
        .args(["fetch", "--depth", "1", "origin", rev])
        .output()
        .context("failed to execute `git fetch`")?;

    let checkout_output = Command::new("git")
        .arg("--git-dir")
        .arg(&git_dir)
        .args(["checkout", rev])
        .current_dir(&directory)
        .output()
        .context("failed to execute `git checkout`")?;
    if !checkout_output.status.success() {
        if !fetch_output.status.success() {
            bail!(
                "failed to fetch revision {} in directory '{}'",
                rev,
                directory.display()
            );
        }
        bail!(
            "failed to checkout revision {} in directory '{}': {}",
            rev,
            directory.display(),
            String::from_utf8_lossy(&checkout_output.stderr)
        );
    }

    Ok(())
}