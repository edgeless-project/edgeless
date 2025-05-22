#!/bin/bash

authors="authors = [ "
first=0
while IFS= read -r line ; do
    if [ $first -eq  0 ] ; then
        first=1
    else
        authors+=", "
    fi
    authors+="\"$line\""
done < <(grep -h  -r SPDX-FileCopyrightText  | egrep "^\/|^#" | cut -d ' ' -f 5- | sort -u)
authors+=" ]"

echo $authors