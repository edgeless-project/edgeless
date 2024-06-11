#!/bin/bash

for i in $(redis-cli --scan --pattern '*' | sort) ; do
	type=$(redis-cli type $i)
	if [ "$type" == "list" ] ; then
		data=$(redis-cli --raw lrange $i 0 -1 | tr '\n' ' ')
	elif [ "$type" == "string" ] ; then
		data=$(redis-cli get $i)
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