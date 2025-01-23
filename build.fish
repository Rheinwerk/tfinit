#!/usr/bin/env fish

set toolchain stable
set targets x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-apple-darwin aarch64-apple-darwin
set targets_installed (rustup target list --installed --toolchain $toolchain)

echo -s "toolchain=" (set_color blue) $toolchain (set_color normal)

# install missing toolchains
for target in $targets
	if not contains $target $targets_installed --toolchain $toolchain
		echo -s "Installing toolchain for " (set_color blue) $target (set_color normal)
		rustup target add  $target --toolchain $toolchain; or return
	end
end

# build for targets
for target in $targets
  echo -s "Building " (set_color blue) $target (set_color normal) " $cargo_cross"
	cargo +$toolchain build --quiet --package tfinit --release --target $target; or return
end

# generate man and completions
echo -sn "Generating " (set_color blue) man (set_color normal)
echo -s " and " (set_color blue) completions (set_color normal)
cargo xtask man
cargo xtask complete

mkdir -p target/upload

for target in $targets
	if not string match -q '*-linux-*' $target
		continue
	end

  echo -s "Packaging deb " (set_color blue) $target (set_color normal) " $cargo_cross"
	cargo deb --no-strip --no-build --package tfinit --target $target --output target/upload/ >/dev/null; or return
end