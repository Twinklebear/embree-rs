$EMBREE_VERSION=$args[0]
$env:EMBREE_DIR="$env:HOME\embree-$EMBREE_VERSION.x64.vc14.windows"

# build the crate
cargo build
if (!$?) {
	exit 1
}

cargo test
if (!$?) {
	exit 1
}

# build the examples
cd examples
Get-ChildItem .\ -Directory | ForEach-Object {
	Write-Output $_
	cd $_
	cargo build
	if (!$?) {
		exit 1
	}
	cd ..
}

