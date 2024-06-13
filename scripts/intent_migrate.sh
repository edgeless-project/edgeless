#!/bin/bash

if [ "$FID" == "" ] ; then
	echo "FID not set"
	exit 1
fi

if [ "$NODE" == "" ] ; then
	echo "NODE not set"
	exit 1
fi

redis-cli set intent:migrate:$FID $NODE
redis-cli lpush intents intent:migrate:$FID
