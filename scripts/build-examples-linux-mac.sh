#!/bin/bash

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

