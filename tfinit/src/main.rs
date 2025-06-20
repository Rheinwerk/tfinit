mod cli;
use anyhow::{bail, Context};
use std::fmt::Formatter;
use std::io::BufRead;
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env("TFINIT_LOG"))
        .init();

    tracing::info!(
        cargo_version = env!("CARGO_PKG_VERSION"),
        git_version = git_version::git_version!(),
    );

    let cli: cli::Cli = clap::Parser::parse();
    tracing::trace!(?cli, "dumping command line arguments");

    let cache_dir: PathBuf = if let Some(cache_dir) = cli.cache_dir {
        tracing::debug!(cache_dir, "using cache dir from command line");
        cache_dir.into()
    } else if let Ok(cache_dir) = std::env::var("TF_PLUGIN_CACHE_DIR") {
        tracing::debug!(cache_dir, "using cache dir from environment variable");
        cache_dir.into()
    } else {
        #[cfg_attr(not(target_os = "windows"), allow(deprecated))]
        let home = std::env::home_dir().context("failed getting home directory")?;

        let terraformrc = std::fs::File::open(home.join(".terraformrc"))
            .context("unable to read ~/.terraformrc")?;
        let hcl: hcl::Body =
            hcl::from_reader(terraformrc).context("unable to parse ~/.terraformrc")?;

        let cache_dir = hcl
            .attributes()
            .find(|a| a.key.as_str() == "plugin_cache_dir")
            .context(r#"no "plugin_cache_dir" attribute in ~/.terraformrc"#)?
            .expr();

        let hcl::Expression::String(cache_dir) = cache_dir else {
            anyhow::bail!(
                "plugin_cache_dir setting cant be used because it is not a string literal"
            );
        };

        tracing::debug!(cache_dir, "using cache dir from environment variable");
        cache_dir.into()
    };

    match (cli.directory, cli.batch) {
        (Some(_), Some(_)) => bail!("-C/--directory and --batch can not be used together"),
        (dir, None) => do_for_path(
            dir.map(PathBuf::from).as_deref(),
            &cache_dir,
            cli.clean,
            cli.dry_run,
        )?,
        (None, Some(paths)) => {
            let base_directory =
                std::env::current_dir().context("failed getting current directory")?;

            for path in read_paths(&paths)? {
                let path = path.context("failed to parse line as path")?;
                let path = PathBuf::from(path);

                let path = if path.is_absolute() {
                    path
                } else {
                    base_directory.join(path)
                };

                do_for_path(Some(&path), &cache_dir, cli.clean, cli.dry_run)?
            }
        }
    }

    Ok(())
}

fn read_paths(path: &str) -> anyhow::Result<Box<dyn Iterator<Item = anyhow::Result<String>>>> {
    if path == "-" {
        return Ok(Box::new(
            std::io::stdin()
                .lines()
                .map(|r| r.map_err(anyhow::Error::from)),
        ));
    }

    let paths = std::fs::File::open(path).context("unable to read batch file")?;
    Ok(Box::new(
        std::io::BufReader::new(paths)
            .lines()
            .map(|r| r.map_err(anyhow::Error::from)),
    ))
}

#[tracing::instrument(skip(cache_dir, clean, dry_run))]
fn do_for_path(
    path: Option<&Path>,
    cache_dir: &Path,
    clean: bool,
    dry_run: bool,
) -> anyhow::Result<()> {
    if let Some(new_cwd) = path {
        std::env::set_current_dir(new_cwd).context("failed changing work directory")?;
        tracing::debug!(
            new_cwd = new_cwd.display().to_string(),
            "changed work directory as requested"
        );
    }

    if clean {
        if let Err(e) = std::fs::remove_dir_all(".terraform") {
            // ignore NotFound errors
            if !matches!(e.kind(), std::io::ErrorKind::NotFound) {
                Err(e).context("failed to remove .terraform directory")?;
            }
        }
    } else {
        anyhow::ensure!(
            !std::fs::exists(".terraform").unwrap_or_default(),
            ".terraform already exists"
        );
    }

    let terraform_files: Vec<_> = glob::glob("*.tf")
        .context("unable to search for terraform files")?
        .filter_map(Result::ok)
        .collect();

    anyhow::ensure!(
        !terraform_files.is_empty(),
        "no terraform files in directory"
    );

    let modules = parse_module_definitions(terraform_files)?;
    let modules_json = ModulesJson::new(modules);

    if !dry_run {
        create_dot_terraform_dir(modules_json, cache_dir)?;
    }

    Ok(())
}

fn parse_module_definitions(files: Vec<PathBuf>) -> anyhow::Result<Vec<Module>> {
    let mut modules = vec![];
    for file in files {
        let file = std::fs::File::open(file).context("terraform files must be readable")?;
        let hcl: hcl::Body = hcl::from_reader(file).context("terraform files must be parseable")?;

        let module_blocks = hcl.blocks().filter(|b| b.identifier.as_str() == "module");
        for module in module_blocks {
            let source = module
                .body
                .attributes()
                .find(|a| a.key.as_str() == "source")
                .context("module block must have a source attribute")?
                .expr();

            let hcl::Expression::String(source) = source else {
                panic!("module source attribute must be a plain string expression");
            };

            let key = module.labels[0].as_str().to_owned();

            let module = Module {
                key,
                dir: source.clone(),
                source: source.clone(),
            };

            tracing::debug!("found module {module}");
            modules.push(module)
        }
    }

    Ok(modules)
}

fn create_dot_terraform_dir(
    modules: ModulesJson,
    cache_dir: &std::path::Path,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(".terraform/modules").context("can not create .terraform directory")?;

    let file = std::fs::File::create(".terraform/modules/modules.json")
        .context("can't create modules.json")?;

    serde_json::to_writer(file, &modules).context("can not write to modules.json")?;

    std::os::unix::fs::symlink(cache_dir, ".terraform/providers")
        .context("cant symlink cache dir")?;

    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct ModulesJson {
    pub modules: Vec<Module>,
}

impl ModulesJson {
    pub fn new(extra_modules: impl IntoIterator<Item = Module>) -> Self {
        let mut modules = vec![Module {
            key: "".to_string(),
            source: "".to_string(),
            dir: ".".to_string(),
        }];
        modules.extend(extra_modules);
        ModulesJson { modules }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct Module {
    pub key: String,
    pub source: String,
    pub dir: String,
}

impl std::fmt::Display for Module {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "module \"{}\" @ {}", self.key, self.dir)
    }
}
