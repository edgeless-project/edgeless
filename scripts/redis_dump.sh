#!/bin/bash

for i in $(redis-cli --scan --pattern '*' | sort) ; do
	data=$(redis-cli get $i)
	if [[ $(grep WRONG <<< $data) != "" ]] ; then
		continue
	fi
	json=$(python3 -m json.tool <<< "$data")
	echo $i
	if [ $? -eq 0 ] ; then
		echo "$json"
	else
		echo $data
	fi
done
