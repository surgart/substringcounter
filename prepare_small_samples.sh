#!/bin/bash

path="./samples/small"
mkdir -p "$path"

printf "aaaa\n%.0s" {1..10} > "$path/aaaa10e1.txt"
printf "aaaa\n%.0s" {1..100} > "$path/aaaa10e2.txt"
printf "aaaa\n%.0s" {1..1000} > "$path/aaaa10e3.txt"

for i in {1..10}; do cat "$path/aaaa10e2.txt" >> "$path/aaaa10e4.txt"; done;
for i in {1..100}; do cat "$path/aaaa10e3.txt" >> "$path/aaaa10e5.txt"; done;
for i in {1..1000}; do cat "$path/aaaa10e3.txt" >> "$path/aaaa10e6.txt"; done;
printf "ababa\n%.0s" {1..1000000} > "$path/ababa10e6.txt"
printf "ababa%.0s" {1..1000000} > "$path/ababa10e6_no_newlines.txt"

printf "abcde%.0s" {1..5432} > "$path/abcde5432_no_newlines.txt"

# Create permission denied file
echo "aaaa" > "$path/permission_denied.txt"
chmod -r "$path/permission_denied.txt"
