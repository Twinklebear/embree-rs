$EMBREE_VERSION=$args[0]
$env:EMBREE_DIR="$env:HOME\embree-$EMBREE_VERSION.x64.vc14.windows"

Write-Output "Travis script debug test"
Write-Host "Travis script debug test"

# build the crate
cargo build
if (!$?) {
	exit 1
}

Write-Output "Built crate"
Write-Host "Built crate"

cargo test
if (!$?) {
	exit 1
}

# build the examples
cd examples
Get-ChildItem .\ -Directory | ForEach-Object {
	Write-Output $_
	Write-Host $_
	cd $_
	cargo build
	if (!$?) {
		exit 1
	}
	cd ..
}

