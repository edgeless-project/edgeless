#!/bin/bash

if [ "$CLI" == "" ] ; then
    CLI=target/debug/edgeless_cli
fi
if [ ! -x "$CLI" ] ; then
    echo "cli not found in '$CLI', specify its path in the CLI environment variable"
    exit 1
fi

rm -f build_functions.log 2> /dev/null
RET=0
for func in $(find functions -type f -name function.json) ; do
    name=$(basename $(dirname $func))
    echo -n "$name: "

    wasm_name="$(dirname $func)/$(grep "\"id\"" $func | cut -f 4 -d '"').wasm"
    if [ -r $wasm_name ] ; then
        echo "OK"
    else
        echo -n "building."
        target/debug/edgeless_cli function build $func >> build_functions.log 2>&1
        if [ "$?" -ne 0 ] ; then
            echo ".FAILED" 
            RET=$(( RET + 1 ))
        else
            echo ".OK"
        fi
    fi
done
exit $RET