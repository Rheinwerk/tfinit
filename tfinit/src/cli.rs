#[derive(clap::Parser, Debug)]
#[clap(name = "tfinit", bin_name = "tfinit")]
/// terraform init for the impatient
///
/// tfinit tries to mimic `terraform init -backend=false`
/// by symlinking the plugin cache dir as a whole
///
/// required versions must already be in the cache for
/// this to work. no validation is done by tfinit
pub struct Cli {
    /// change working directory
    #[clap(short = 'C', long)]
    pub directory: Option<String>,

    /// override cache dir
    ///
    /// when no value is provided "plugin_cache" in ~/.terraformrc is used
    #[clap(long, env("TF_PLUGIN_CACHE_DIR"))]
    pub cache_dir: Option<String>,

    /// remove .terraform directory
    #[clap(short, long)]
    pub clean: bool,

    /// don't modify files
    #[clap(short = 'n', long, conflicts_with = "clean")]
    pub dry_run: bool,
}
