$env:WORK_DIR=(get-location)
$env:EMBREE_DIR="${env:WORK_DIR}\embree-${env:EMBREE_VERSION}.x64.vc14.windows\"

Write-Output "Building embree-rs"
cargo build
if (!$?) {
    exit 1
}

Write-Output "Running embree-rs Tests"
cargo test
if (!$?) {
    exit 1
}

# build the examples
cd examples
#Get-ChildItem .\ -Directory | ForEach-Object {
#	Write-Output $_
	#cd $_
    cd triangle
	cargo build
	if (!$?) {
		exit 1
	}
	cd ..
#}

