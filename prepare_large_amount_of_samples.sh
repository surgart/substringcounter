#!/bin/bash

path="./samples/multiple"
mkdir "$path"

printf "ababa%.0s" {1..1000000} > "$path/ababa10e6_0000_no_newlines.txt"
for i in $(seq -f "%04g" 1 1000); do cp "$path/ababa10e6_0000_no_newlines.txt" "$path/ababa10e6_${i}_no_newlines.txt"; done;