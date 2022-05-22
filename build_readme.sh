#!/bin/sh
grep -E "\/\/\!." ./src/lib.rs | cut -c 5- > README.md
