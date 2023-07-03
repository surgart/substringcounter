#!/bin/bash

path="./samples/large"
mkdir -p "$path"
printf "ababa%.0s" {1..1000000} > "$path/ababa10e6_no_newlines.txt"
for i in {1..100}; do cat "$path/ababa10e6_no_newlines.txt" >> "$path/ababa10e8_no_newlines.txt"; done;
for i in {1..10}; do cat "$path/ababa10e8_no_newlines.txt" >> "$path/ababa10e9_no_newlines.txt"; done;
