#!/bin/bash

if [ "$REDIS_PORT" == "" ] ; then
	REDIS_PORT=6379
fi

for i in $(redis-cli -p $REDIS_PORT --scan --pattern '*' | sort) ; do
	type=$(redis-cli -p $REDIS_PORT type $i)
	if [ "$type" == "list" ] ; then
		data=$(redis-cli -p $REDIS_PORT --raw lrange $i 0 -1 | tr '\n' ' ')
	elif [ "$type" == "string" ] ; then
		data=$(redis-cli -p $REDIS_PORT get $i)
		json=$(python3 -m json.tool <<< "$data")
		if [ $? -eq 0 ] ; then
			data=$json
		fi
	else
		continue
	fi
	echo $i
	echo $data
done