# tfinit

`terraform init -backend=false` for the impatient

`tfinit` mimics `terraform init -backend=false` for fast initialization in CI contexts. Note that `tfinit` does not
install any plugins. The plugins have to be in the cache.

**Steps**

- (optional) remove `.terraform/` directory (`--clean`)
- symlink `.terraform/providers` to
  the [terraform plugin cache](https://developer.hashicorp.com/terraform/cli/config/config-file#provider-plugin-cache)
- write `.terraform/modules/modules.json`

Get the docker image `docker pull docker.io/rheinwerk/tfinit:0.1.0` or build locally.

## Build

There are multiple ways

- just build the binary, via cargo
- build a docker container (see below)
- build debian packages (see below)

### Docker

The `Dockerfile` contains a multi-stage build that results in a debian image with the binary as the entrypoint.

`docker build --platform=linux/arm64,linux/amd64 -t tfinit .`

### Debian Packages

`build.fish` installs rustup targets, generates man and completions, builds for multiple architectures and places debian
packages in `target/upload`. The individual steps are as follows:

- `cargo xtask man` generates man pages in `target/man` (uses [cargo-xtask](https://github.com/matklad/cargo-xtask))
- `cargo xtask complete` generates completions in `target/complete` (
  uses [cargo-xtask](https://github.com/matklad/cargo-xtask))
- `cargo build` (or [cross](https://github.com/cross-rs/cross) build)
- `cargo deb` (via [cargo-deb](https://github.com/kornelski/cargo-deb))
 