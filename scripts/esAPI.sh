#!/bin/bash


curl -X GET "http://localhost:9200/node_health_test/_search?pretty" -u "elastic:edgeless" -H "Content-Type: application/json" -d '{"query": {"match_all": {}}}'

curl -X GET "http://localhost:9200/domain_info/_search?pretty" -u "edgeless:5T^97^QiR2?t" -H "Content-Type: application/json" -d '{"query": {"match_all": {}}}'

curl -X POST "http://localhost:9200/node_health_test/_delete_by_query" -H "Content-Type: application/json" -d '{
  "query": {
    "match_all": {}
  }
}'