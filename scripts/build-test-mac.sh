#!/bin/bash

source embree-${EMBREE_VERSION}.x86_64.macosx/embree-vars.sh
echo "Building embree-rs tests"
cargo test
if [[ "$?" != "0" ]]; then
    exit 1
fi

# build the examples
cd examples
for d in `ls ./`; do
	cd $d
	pwd
	cargo build
	if [[ "$?" != "0" ]]; then
		exit 1
	fi
	cd ../
done

