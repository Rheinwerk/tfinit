use clap::CommandFactory;

fn main() {
    let arg1 = std::env::args().nth(1);
    match arg1.as_deref() {
        Some("man") => generate_man(),
        Some("complete") => generate_completion(),
        Some(other) => eprintln!("unknown option: {}", other),
        None => eprintln!("nothing to do"),
    }

    std::process::exit(1);
}

fn generate_man() {
    std::fs::create_dir_all("target/man").unwrap();

    let cmd = tfinit::Cli::command().name("tfinit").bin_name("tfinit");
    clap_mangen::generate_to(cmd, "target/man").unwrap();

    std::process::exit(0);
}

fn generate_completion() {
    use clap_complete::Shell::*;

    std::fs::create_dir_all("target/complete").unwrap();

    for shell in [Bash, Zsh, Fish] {
        clap_complete::generate_to(
            shell,
            &mut tfinit::Cli::command(),
            "tfinit",
            "target/complete",
        )
        .unwrap();
    }

    std::process::exit(0);
}

mod tfinit {
    include!("../../tfinit/src/cli.rs");
}
