#!/bin/bash

# SPDX-FileCopyrightText: © 2023 Siemens AG
# SPDX-License-Identifier: MIT

included_files="(\.rs|\.toml|\.proto)"
spdx_header="SPDX-License-Identifier"

# on MacOS install gnu-sed with brew and use gsed instead of sed - macOS version won't work

find $1 -type f | while read -r file; do
    filename=$(basename "$file")
    if [[ $filename =~ $included_files ]]; then
        # TODO: if any spdx header is present, ignore to avoid duplication /
        # remove it first

        extension=$(echo "$filename" | awk -F. '{print $NF}')
        date=$(git log --follow --format=%ad --date=format:'%Y' $file | tail -1) # date of creation
        authors=$(git blame --line-porcelain $file | sed -n 's/author //p' | sort | uniq -c | sort -rn)

        # specify appropriate comment prefix for file extension
        prefix="//"
        case $extension in
            "toml")
            prefix="#"
            ;;
        esac

        # insert the license header
        gsed -i "1i $prefix SPDX-License-Identifier: MIT" -- $file

        # if contains Mortier
        if [[ $authors =~ "Mortier" ]]; then
            # SPDX-FileCopyrightText: © 2023 Richard Mortier <richard.mortier@cl.cam.ac.uk>
            gsed -i "1i $prefix SPDX-FileCopyrightText: © $date Richard Mortier <richard.mortier@cl.cam.ac.uk>" $file
        fi

        # if contains lukasz-zet / Zalewski string / Sauer
        if [[ $authors =~ "Zalewski" || $authors =~ "lukasz-zet" || $authors =~ "Sauer" ]]; then
            gsed -i "1i $prefix SPDX-FileCopyrightText: © $date Siemens AG" $file
        fi

        # if contains Claudio string - second biggest contributor
        if [[ $authors =~ "Claudio" ]]; then
            gsed -i "1i $prefix SPDX-FileCopyrightText: © $date Claudio Cicconetti <c.cicconetti@iit.cnr.it>" $file
        fi

        # if contains Raphael string - keep it on the top
        if [[ $authors =~ "Raphael" ]]; then
            gsed -i "1i $prefix SPDX-FileCopyrightText: © $date TUM" $file
        fi

    else
        echo "ignored file $filename"
    fi
done