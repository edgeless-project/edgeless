// Function to test the connection to Elasticsearch
async function testElasticsearchConnection(client) {
  const maxRetries = 10; // Set a maximum number of retries
  let attempt = 0;

  while (attempt < maxRetries) {
    try {
      const health = await client.cluster.health();
      console.log('Elasticsearch cluster health:', health.body);
      return; // Connection successful, exit the function
    } catch (error) {
      attempt++;
      console.error(`Attempt ${attempt} failed: Error connecting to Elasticsearch:`, error.message);

      if (attempt >= maxRetries) {
        console.error('Max retries reached. Could not connect to Elasticsearch.');
        return; // Exit after max retries
      }

      // Wait before retrying (e.g., 5 seconds)
      console.log(`Retrying in 5 seconds...`);
      await new Promise(resolve => setTimeout(resolve, 5000)); // 5000 ms = 5 seconds
    }
  }
}
  // apiController.js

const getDomainHealth = async (req, res) => {
  try {
    const result = await req.client.search({
      index: 'node_health_test',
      body: {
        query: {
          match_all: {}
        }
      }
    });
    res.json(result.body);
  } catch (error) {
    console.error(error);
    res.status(500).send('Error connecting to Elasticsearch');
  }
};

const getDomainProviders = async (req, res) => {
  try {
    const result = await req.client.search({
      index: 'provider',
      body: {
        query: {
          match_all: {}
        }
      }
    });
    res.json(result.body);
  } catch (error) {
    console.error(error);
    res.status(500).send('Error connecting to Elasticsearch');
  }
};

const getDomainDependencies = async (req, res) => {
  try {
    const result = await req.client.search({
      index: 'dependency',
      body: {
        query: {
          match_all: {}
        }
      }
    });
    res.json(result.body);
  } catch (error) {
    console.error(error);
    res.status(500).send('Error connecting to Elasticsearch');
  }
};

const getDomainInstance = async (req, res) => {
  try {
    const result = await req.client.search({
      index: 'instance',
      body: {
        query: {
          match_all: {}
        }
      }
    });
    res.json(result.body);
  } catch (error) {
    console.error(error);
    res.status(500).send('Error connecting to Elasticsearch');
  }
};




// Function to search for a specific node_id
const getNodeCapabilities = async (req, res) => {
  try {
    const nodeId = req.params.nodeId;
    const result = await req.client.search({
      index: 'node_capabilities',
      body: {
        query: {
          match: {
            node_id: nodeId
          }
        }
      }
    });
    res.json(result.body);
  } catch (error) {
    console.error('Error searching for node_id:', error);
    res.status(500).send('Error connecting to Elasticsearch');
  }
};


// Function to search for a specific node_id
const getNodeHealth = async (req, res) => {
  try {
    const nodeId = req.params.nodeId;
    let { start, end } = req.query;

    // Get current timestamp
    const now = new Date();

    // Default time range: last 1 hour
    if (!start) {
      start = new Date(now.getTime() - 60 * 60 * 1000).toISOString(); // 1 hour ago
    }
    if (!end) {
      end = now.toISOString(); // Current time
    }

    const result = await req.client.search({
      index: 'node_health_test',
      body: {
        query: {
          bool: {
            must: [
              { match: { node_id: nodeId } },
              {
                range: {
                  timestamp: {
                    gte: start,
                    lte: end,
                    format: "strict_date_optional_time"
                  }
                }
              }
            ]
          }
        }
      }
    });

    console.log('Elasticsearch response:', result);
    res.json(result.body);
  } catch (error) {
    console.error('Error searching for node_id:', error);
    res.status(500).send('Error connecting to Elasticsearch');
  }
};

const getNodeHealthAvg = async (req, res) => {
  try {
    const nodeId = req.params.nodeId;


    const nodeExistResult = await req.client.search({
      index: 'node_health_test',
      body: {
        query: {
          match: {
            node_id: nodeId
          }
        }
      }
    });
    console.log('Node Exists response:', nodeExistResult);

    if (!nodeExistResult.body.hits.hits || !nodeExistResult.body.hits.total) {
      return res.status(404).json({ message: "Hits not found." });
    }
    if (nodeExistResult.body.hits.total.value === 0) {
      return res.status(404).json({ message: "Node not found." });
    }

    // Proceed with the aggregation query
    const result = await req.client.search({
      index: 'node_health_test',
      body: {
        query: {
          match: {
            node_id: nodeId,
          }
        },
        aggs: {
          avg_cpu_usage: { avg: { field: 'proc_cpu_usage' } },
          avg_memory_used: { avg: { field: 'mem_used' } },
          avg_rx_bytes: { avg: { field: 'tot_rx_bytes' } },
          avg_tx_bytes: { avg: { field: 'tot_tx_bytes' } }
        },
        size: 0
      }
    });

    console.log('Elasticsearch response:', result);

    const response = {
      avg_cpu: result.body.aggregations.avg_cpu_usage?.value || 0,
      avg_memory: result.body.aggregations.avg_memory_used?.value || 0,
      avg_rx: result.body.aggregations.avg_rx_bytes?.value || 0,
      avg_tx: result.body.aggregations.avg_tx_bytes?.value || 0,
    };

    return res.status(200).json(response);
  } catch (error) {
    console.error("Error fetching node health data:", error);
    return res.status(500).json({ message: "Internal server error." });
  }
};

const getNodeHealthRollingAvg = async (req, res) => {
  try {
    const nodeId = req.params.nodeId;
    const windowSize = '1m'; //Rolling window size

    const result = await req.client.search({
      index: 'node_health_test',
      body: {
        query: {
          match: {
            node_id: nodeId,
          },
        },
        aggs: {
          by_time: {
            date_histogram: {
              field: 'timestamp',
              fixed_interval: windowSize,
              min_doc_count: 0,
            },
            aggs: {
              avg_cpu_usage: {
                avg: {
                  field: 'proc_cpu_usage',
                },
              },
              avg_memory_used: {
                avg: {
                  field: 'mem_used',
                },
              },
              avg_rx_bytes: {
                avg: {
                  field: 'tot_rx_bytes',
                },
              },
              avg_tx_bytes: {
                avg: {
                  field: 'tot_tx_bytes',
                },
              },
              rolling_avg_cpu: {
                moving_fn: {
                  buckets_path: 'avg_cpu_usage',
                  window: 5,
                  script: 'MovingFunctions.unweightedAvg(values)',
                },
              },
              rolling_avg_mem: {
                moving_fn: {
                  buckets_path: 'avg_memory_used',
                  window: 5,
                  script: 'MovingFunctions.unweightedAvg(values)',
                },
              },
              rolling_avg_rx: {
                moving_fn: {
                  buckets_path: 'avg_rx_bytes',
                  window: 5,
                  script: 'MovingFunctions.unweightedAvg(values)',
                },
              },
              rolling_avg_tx: {
                moving_fn: {
                  buckets_path: 'avg_tx_bytes',
                  window: 5,
                  script: 'MovingFunctions.unweightedAvg(values)',
                },
              },
            },
          },
        },
      },
    });

    // Process and return the result
    res.status(200).json({
      success: true,
      data: result.body.aggregations.by_time.buckets,
    });
  } catch (error) {
    console.error('Error fetching node health data:', error);
    res.status(500).json({
      success: false,
      error: 'Error fetching node health data',
    });
  }
};

const getMaxConsumptionNode = async (req, res) => {
  try {
    const response = await req.client.search({
      index: 'node_health_test',
      size: 0, // only need aggregations, not top-level documents
      body: {
        aggs: {
          max_cpu_usage: {
            max: {
              field: 'proc_cpu_usage'
            }
          },
          max_cpu_node: {
            top_hits: {
              sort: [
                { proc_cpu_usage: { order: 'desc' } }
              ],
              _source: {
                includes: ['node_id', 'proc_cpu_usage']
              },
              size: 1 // Fetch the top document
            }
          },
          max_mem_used: {
            max: {
              field: 'mem_used'
            }
          },
          max_mem_node: {
            top_hits: {
              sort: [
                { mem_used: { order: 'desc' } }
              ],
              _source: {
                includes: ['node_id', 'mem_used']
              },
              size: 1
            }
          },
          max_memory: {
            max: {
              field: 'proc_memory'
            }
          },
          max_memory_node: {
            top_hits: {
              sort: [
                { proc_memory: { order: 'desc' } }
              ],
              _source: {
                includes: ['node_id', 'proc_memory']
              },
              size: 1
            }
          }
        }
      }
    });

    // Extract aggregation results
    const maxCpuUsage = response.body.aggregations.max_cpu_usage.value;
    const maxCpuNode = response.body.aggregations.max_cpu_node.hits.hits[0]?._source;

    const maxMemUsed = response.body.aggregations.max_mem_used.value;
    const maxMemNode = response.body.aggregations.max_mem_node.hits.hits[0]?._source;

    const maxMemory = response.body.aggregations.max_memory.value;
    const maxMemoryNode = response.body.aggregations.max_memory_node.hits.hits[0]?._source;

    // Create a response object
    const result = {
      max_cpu_usage: maxCpuUsage,
      max_cpu_node: maxCpuNode,
      max_mem_used: maxMemUsed,
      max_mem_node: maxMemNode,
      max_memory: maxMemory,
      max_memory_node: maxMemoryNode
    };

    // Return the response to the client
    res.json({
      success: true,
      data: result
    });
  } catch (error) {
    console.error('Error executing Elasticsearch query:', error);
    res.status(500).json({
      success: false,
      message: 'Error executing Elasticsearch query',
      error: error.message
    });
  }
};

const getMinConsumptionNode = async (req, res) => {
    try {
      const response = await req.client.search({
        index: 'node_health_test',
        size: 0, // only need aggregations, not top-level documents
        body: {
          aggs: {
            min_cpu_usage: {
              min: {
                field: 'proc_cpu_usage'
              }
            },
            min_cpu_node: {
              top_hits: {
                sort: [
                  { proc_cpu_usage: { order: 'asc' } } // Sort ascending to get the minimum value
                ],
                _source: {
                  includes: ['node_id', 'proc_cpu_usage']
                },
                size: 1 // Fetch the top document
              }
            },
            min_mem_used: {
              min: {
                field: 'mem_used'
              }
            },
            min_mem_node: {
              top_hits: {
                sort: [
                  { mem_used: { order: 'asc' } } // Sort ascending to get the minimum value
                ],
                _source: {
                  includes: ['node_id', 'mem_used']
                },
                size: 1
              }
            },
            min_memory: {
              min: {
                field: 'proc_memory'
              }
            },
            min_memory_node: {
              top_hits: {
                sort: [
                  { proc_memory: { order: 'asc' } } // Sort ascending to get the minimum value
                ],
                _source: {
                  includes: ['node_id', 'proc_memory']
                },
                size: 1
              }
            }
          }
        }
      });
  
      // Extract aggregation results
      const minCpuUsage = response.body.aggregations.min_cpu_usage.value;
      const minCpuNode = response.body.aggregations.min_cpu_node.hits.hits[0]?._source;
  
      const minMemUsed = response.body.aggregations.min_mem_used.value;
      const minMemNode = response.body.aggregations.min_mem_node.hits.hits[0]?._source;
  
      const minMemory = response.body.aggregations.min_memory.value;
      const minMemoryNode = response.body.aggregations.min_memory_node.hits.hits[0]?._source;
  
      // Create a response object
      const result = {
        min_cpu_usage: minCpuUsage,
        min_cpu_node: minCpuNode,
        min_mem_used: minMemUsed,
        min_mem_node: minMemNode,
        min_memory: minMemory,
        min_memory_node: minMemoryNode
      };
  
      // Return the response to the client
      res.json({
        success: true,
        data: result
      });
    } catch (error) {
      console.error('Error executing Elasticsearch query:', error);
      res.status(500).json({
        success: false,
        message: 'Error executing Elasticsearch query',
        error: error.message
      });
    }
  };

const getNodeFunctionSamples = async (req, res) => {
  console.log('getNodeFunctionSamples');
  //TODO: Implement this function
};

const getAvgFunctionExecutionTime = async(req, res) => {
  console.log('getAvgFunctionExecutionTime');
  //TODO: Implement this function
};

const getRollingAvgFunctionExecutionTime = async(req, res) => {
  console.log('getRollingAvgFunctionExecutionTime');
  //TODO: Implement this function
};

const postGpuMetrics = async (req, res) => {
  try {
    const {cpu, temp } = req.body;
    const result = await req.client.index({
      index: 'gpu_metrics',
      body: JSON.stringify({
      // node_id: nodeId,
      gpu_usage: cpu,
      gpu_temp: temp,
      timestamp: new Date().toISOString().replace('T', ' ').replace(/\.\d+Z$/, '')
      })
    });
    res.json(result.body);
  } catch (error) {
    console.error('Error posting GPU metrics:', error);
    res.status(500).send('Error connecting to Elasticsearch');
  }
};
const getGpuMetrics = async (req, res) => {
  try {
    const { startTime, endTime } = req.query;

    //Es query
    const query = {
      index: 'gpu_metrics',
      body: {
        query: {
          range: {
            timestamp: {
              gte: startTime || 'now-1d/d', //last 24 hours if not provided
              lte: endTime || 'now', //current time if not provided
              format: 'strict_date_optional_time'
            }
          }
        },
        sort: [
          { timestamp: { order: 'desc' } } // Sort by timestamp, descending
        ]
      }
    };

    const result = await req.client.search(query);

    //extract relevant data
    const metrics = result.body.hits.hits.map(hit => ({
      timestamp: hit._source.timestamp,
      cpu: hit._source.gpu_usage,
      temp: hit._source.gpu_temp
    }));

    //send response
    res.json({
      success: true,
      metrics
    });
  } catch (error) {
    console.error('Error retrieving GPU metrics:', error);
    res.status(500).json({
      success: false,
      message: 'Error retrieving GPU metrics',
      error: error.message
    });
  }
};
module.exports = {
  testElasticsearchConnection,
  getDomainHealth,
  getDomainProviders,
  getNodeCapabilities,
  getDomainDependencies,
  getDomainInstance,
  getNodeHealth,
  getNodeHealthAvg,
  getNodeHealthRollingAvg,
  getMaxConsumptionNode,
  getMinConsumptionNode,
  postGpuMetrics,
  getGpuMetrics,
  getNodeFunctionSamples,
  getAvgFunctionExecutionTime,
  getRollingAvgFunctionExecutionTime
};


