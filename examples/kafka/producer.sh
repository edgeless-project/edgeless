#!/bin/bash

if [ "$PERIOD" == "" ] ; then
    PERIOD=0.1
fi

if [ "$DURATION" == "" ] ; then
    DURATION=60
fi

if [ "$OUTFILE" == "" ] ; then
    OUTFILE=edgeless.prod
fi

SECONDS=0
COUNTER=0
while (true) ; do
    if [ $SECONDS -ge $DURATION ] ; then
        break
    fi

    echo "$COUNTER" | curl -H "Host: kafka" -XPOST http://127.0.0.1:7035/hello -d@-
    echo "$(date +%s.%N) $COUNTER" >> $OUTFILE
    COUNTER=$(( COUNTER + 1 ))

    sleep $PERIOD
done