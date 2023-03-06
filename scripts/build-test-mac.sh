#!/bin/bash

source embree-${EMBREE_VERSION}.x86_64.macosx/embree-vars.sh
echo "Building embree-rs tests"
cargo test
if [[ "$?" != "0" ]]; then
    exit 1
fi

# build the examples
cd examples
for dir in */; do
	if [[ -d "$dir" && "$dir" != "todos/" ]]; then
	  cd $dir
	  pwd
	  cargo build
	  if [[ "$?" != "0" ]]; then
	    exit 1
	  fi
	  cd ../
	fi
done

