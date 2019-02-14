#!/bin/bash
cargo build
cargo test
cargo doc

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

