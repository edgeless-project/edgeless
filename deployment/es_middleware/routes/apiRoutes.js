const express = require('express');
const router = express.Router();
const controller = require('../controller/apiController');



//return GET /node_health/_search index
/**
 * @swagger
 * /getDomainHealth:
 *   get:
 *     tags:
 *       - Domain Information
 *     summary: Get domain health
 *     responses:
 *       200:
 *         description: Returns the domain health
 */
router.get('/getDomainHealth', controller.getDomainHealth);

/**
 * @swagger
 * /getDomainProviders:
 *   get:
 *     tags:
 *       - Domain Information
 *     summary: Get domain providers
 *     responses:
 *       200:
 *         description: Returns the domain providers
 */
//return GET /provider/_search index
router.get('/getDomainProviders', controller.getDomainProviders);

/**
 * @swagger
 * /getDomainDependency:
 *   get:
 *     summary: Get domain dependency
 *     tags:
 *       - Domain Information
 *     responses:
 *       200:
 *         description: Returns the domain dependencies
 */
//return GET /dependency/_search index
router.get('/getDomainDependency', controller.getDomainDependencies);

/**
 * @swagger
 * /getDomainInstance:
 *   get:
 *     summary: Get domain instance
 *     tags:
 *       - Domain Information
 *     responses:
 *       200:
 *         description: Returns the domain instance
 */
//return GET /instance/_search index
router.get('/getDomainInstance', controller.getDomainInstance);

/**
 * @swagger
 * /getNodeCapabilities/{nodeId}:
 *   get:
 *     summary: Get node capabilities by Node ID
 *     description: Retrieves the capabilities of a specific node by its ID.
 *     tags:
 *       - Node Capabilities
 *     parameters:
 *       - in: path
 *         name: nodeId
 *         required: true
 *         schema:
 *           type: string
 *         description: The ID of the node
 *     responses:
 *       200:
 *         description: Successfully retrieved node capabilities
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               example: 
 *                 nodeId: 42f8853c-2d51-4f14-b22a-4ce74d62e18f
 *                 capabilities: ["capability1", "capability2"]
 *       404:
 *         description: Node not found
 */
//return GET /node_capabilities/_search index
router.get('/getNodeCapabilities/:nodeId', controller.getNodeCapabilities);

/**
 * @swagger
 * /getNodeHealth/{nodeId}:
 *   get:
 *     summary: Get node health by Node ID
 *     description: Retrieves the health status of a specific node by its ID.
 *     tags:
 *       - Node Health
 *     parameters:
 *       - in: path
 *         name: nodeId
 *         required: true
 *         schema:
 *           type: string
 *         description: The ID of the node
 *     responses:
 *       200:
 *         description: Successfully retrieved node health
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               example: 
 *                 nodeId: fda6ce79-46df-4f96-a0d2-456f720f606c
 *                 health: "healthy"
 *       404:
 *         description: Node not found
 */
//return GET /node_health/_search index
router.get('/getNodeHealth/:nodeId', controller.getNodeHealth);

/**
 * @swagger
 * /getAvgNodeHealth/{nodeId}:
 *   get:
 *     summary: Retrieve average node health metrics
 *     description: Fetches the average CPU usage, memory usage, received bytes (rx), and transmitted bytes (tx) for a specific node identified by its `nodeId`.
 *     tags:
 *       - Node Health
 *     parameters:
 *       - name: nodeId
 *         in: path
 *         required: true
 *         description: The unique identifier of the node.
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Successfully retrieved average node health metrics.
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 avg_cpu_usage:
 *                   type: number
 *                   format: float
 *                   description: Average CPU usage.
 *                 avg_memory_used:
 *                   type: number
 *                   format: float
 *                   description: Average memory usage in bytes.
 *                 avg_rx_bytes:
 *                   type: number
 *                   format: float
 *                   description: Average received bytes.
 *                 avg_tx_bytes:
 *                   type: number
 *                   format: float
 *                   description: Average transmitted bytes.
 *       400:
 *         description: Bad request due to invalid or missing `nodeId`.
 *       404:
 *         description: Node not found or no data available for the specified `nodeId`.
 *       500:
 *         description: Internal server error while processing the request.
 */

router.get('/getAvgNodeHealth/:nodeId', controller.getNodeHealthAvg);


/**
 * @swagger
 * /getRollingAvgNodeHealth/{nodeId}:
 *   get:
 *     tags:
 *       - Node Health
 *     summary: Get rolling average of node health
 *     description: Retrieves the rolling average of health metrics for a specific node over a defined period.
 *     parameters:
 *       - name: nodeId
 *         in: path
 *         required: true
 *         description: The unique identifier of the node.
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Successfully retrieved the rolling average of node health.
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 nodeId:
 *                   type: string
 *                   description: The unique identifier of the node.
 *                 rollingAverage:
 *                   type: number
 *                   format: float
 *                   description: The calculated rolling average of the node's health.
 *       400:
 *         description: Bad request - Invalid or missing nodeId.
 *       404:
 *         description: Node not found.
 *       500:
 *         description: Internal server error.
 */
router.get('/getRollingAvgNodeHealth/:nodeId', controller.getNodeHealthRollingAvg);


/**
 * @swagger
 * /getMaxConsumptionNode:
 *   get:
 *     summary: Get Node with Maximum Resource Consumption
 *     description: Fetch the node(s) with the highest CPU usage, memory usage, and memory capacity from Elasticsearch.
 *     tags:
 *       - Node Consumption
 *     responses:
 *       200:
 *         description: Successfully retrieved maximum consumption node details.
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 success:
 *                   type: boolean
 *                   description: Indicates if the operation was successful.
 *                   example: true
 *                 data:
 *                   type: object
 *                   properties:
 *                     max_cpu_usage:
 *                       type: number
 *                       format: float
 *                       description: Maximum CPU usage percentage.
 *                       example: 85
 *                     max_cpu_node:
 *                       type: object
 *                       description: Details of the node with maximum CPU usage.
 *                       properties:
 *                         node_id:
 *                           type: string
 *                           description: Identifier of the node.
 *                           example: "node-123"
 *                         proc_cpu_usage:
 *                           type: number
 *                           format: float
 *                           description: CPU usage percentage of the node.
 *                           example: 85
 *                     max_mem_used:
 *                       type: number
 *                       format: float
 *                       description: Maximum memory used (in bytes or GB depending on your data).
 *                       example: 8589934592
 *                     max_mem_node:
 *                       type: object
 *                       description: Details of the node with maximum memory usage.
 *                       properties:
 *                         node_id:
 *                           type: string
 *                           description: Identifier of the node.
 *                           example: "node-456"
 *                         mem_used:
 *                           type: number
 *                           format: float
 *                           description: Memory usage of the node.
 *                           example: 8589934592
 *                     max_memory:
 *                       type: number
 *                       format: float
 *                       description: Maximum memory capacity (in bytes or GB depending on your data).
 *                       example: 17179869184
 *                     max_memory_node:
 *                       type: object
 *                       description: Details of the node with maximum memory capacity.
 *                       properties:
 *                         node_id:
 *                           type: string
 *                           description: Identifier of the node.
 *                           example: "node-789"
 *                         proc_memory:
 *                           type: number
 *                           format: float
 *                           description: Memory capacity of the node.
 *                           example: 17179869184
 *       500:
 *         description: Internal server error occurred while fetching data.
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 success:
 *                   type: boolean
 *                   description: Indicates if the operation was successful.
 *                   example: false
 *                 error:
 *                   type: string
 *                   description: Error message.
 *                   example: "Error executing Elasticsearch query"
 */
router.get("/getMaxConsumptionNode", controller.getMaxConsumptionNode);

/**
 * @swagger
 * /getMinConsumptionNode:
 *   get:
 *     summary: Get Node with Minimum Resource Consumption
 *     description: Fetch the node(s) with the lowest CPU usage, memory usage, and memory capacity from Elasticsearch.
 *     tags:
 *       - Node Consumption
 *     responses:
 *       200:
 *         description: Successfully retrieved minimum consumption node details.
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 success:
 *                   type: boolean
 *                   description: Indicates if the operation was successful.
 *                   example: true
 *                 data:
 *                   type: object
 *                   properties:
 *                     min_cpu_usage:
 *                       type: number
 *                       format: float
 *                       description: Minimum CPU usage percentage.
 *                       example: 5
 *                     min_cpu_node:
 *                       type: object
 *                       description: Details of the node with minimum CPU usage.
 *                       properties:
 *                         node_id:
 *                           type: string
 *                           description: Identifier of the node.
 *                           example: "node-101"
 *                         proc_cpu_usage:
 *                           type: number
 *                           format: float
 *                           description: CPU usage percentage of the node.
 *                           example: 5
 *                     min_mem_used:
 *                       type: number
 *                       format: float
 *                       description: Minimum memory used (in bytes or GB depending on your data).
 *                       example: 1073741824
 *                     min_mem_node:
 *                       type: object
 *                       description: Details of the node with minimum memory usage.
 *                       properties:
 *                         node_id:
 *                           type: string
 *                           description: Identifier of the node.
 *                           example: "node-202"
 *                         mem_used:
 *                           type: number
 *                           format: float
 *                           description: Memory usage of the node.
 *                           example: 1073741824
 *                     min_memory:
 *                       type: number
 *                       format: float
 *                       description: Minimum memory capacity (in bytes or GB depending on your data).
 *                       example: 2147483648
 *                     min_memory_node:
 *                       type: object
 *                       description: Details of the node with minimum memory capacity.
 *                       properties:
 *                         node_id:
 *                           type: string
 *                           description: Identifier of the node.
 *                           example: "node-303"
 *                         proc_memory:
 *                           type: number
 *                           format: float
 *                           description: Memory capacity of the node.
 *                           example: 2147483648
 *       500:
 *         description: Internal server error occurred while fetching data.
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 success:
 *                   type: boolean
 *                   description: Indicates if the operation was successful.
 *                   example: false
 *                 error:
 *                   type: string
 *                   description: Error message.
 *                   example: "Error executing Elasticsearch query"
 */
router.get("/getMinConsumptionNode", controller.getMinConsumptionNode);
/**
 * @swagger
 * /getNodeFunctionSamples/{nodeId}:
 *   get:
 *     tags:
 *       - Node Performance Samples
 *     summary: Get function execution samples for a node
 *     description: Retrieves a list of function samples associated with a specific node.
 *     parameters:
 *       - name: nodeId
 *         in: path
 *         required: true
 *         description: The unique identifier of the node.
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Successfully retrieved the function samples for the node.
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 nodeId:
 *                   type: string
 *                   description: The unique identifier of the node.
 *                 functionSamples:
 *                   type: array
 *                   description: A list of function samples for the specified node.
 *                   items:
 *                     type: object
 *                     properties:
 *                       functionName:
 *                         type: string
 *                         description: The name of the function.
 *                       sampleData:
 *                         type: object
 *                         additionalProperties: true
 *                         description: The sample data associated with the function.
 *       400:
 *         description: Bad request - Invalid or missing nodeId.
 *       404:
 *         description: Node not found.
 *       500:
 *         description: Internal server error.
 */
router.get('/getNodeFunctionSamples/:nodeId', controller.getNodeFunctionSamples);

/**
 * @swagger
 * /getAvgFunctionExecutionTime/{nodeId}:
 *   get:
 *     tags:
 *       - Node Performance Samples
 *     summary: Get average execution time of function samples
 *     description: Retrieves the average execution time of all function samples for a specific node.
 *     parameters:
 *       - name: nodeId
 *         in: path
 *         required: true
 *         description: The unique identifier of the node.
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Successfully retrieved the average execution time of function samples.
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 nodeId:
 *                   type: string
 *                   description: The unique identifier of the node.
 *                 avgExecutionTime:
 *                   type: number
 *                   format: float
 *                   description: The average execution time (in milliseconds) of the node's function samples.
 *       400:
 *         description: Bad request - Invalid or missing nodeId.
 *       404:
 *         description: Node not found.
 *       500:
 *         description: Internal server error.
 */
router.get('/getAvgFunctionExecutionTime/:nodeId', controller.getAvgFunctionExecutionTime);

/**
 * @swagger
 * /getRollingAvgFunctionExecutionTime/{nodeId}:
 *   get:
 *     tags:
 *       - Node Performance Samples
 *     summary: Get rolling average execution time of function samples
 *     description: Retrieves the rolling average execution time of function samples for a specific node over a defined period.
 *     parameters:
 *       - name: nodeId
 *         in: path
 *         required: true
 *         description: The unique identifier of the node.
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Successfully retrieved the rolling average execution time of function samples.
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 nodeId:
 *                   type: string
 *                   description: The unique identifier of the node.
 *                 rollingAvgExecutionTime:
 *                   type: number
 *                   format: float
 *                   description: The rolling average execution time (in milliseconds) of the node's function samples.
 *       400:
 *         description: Bad request - Invalid or missing nodeId.
 *       404:
 *         description: Node not found.
 *       500:
 *         description: Internal server error.
 */
router.get('/getRollingAvgFunctionExecutionTime/:nodeId', controller.getRollingAvgFunctionExecutionTime);
/**
 * @swagger
 * /postGpuMetrics:
 *   post:
 *     summary: Post GPU metrics
 *     description: Receives GPU metrics (CPU, temp) and posts them to an Elasticsearch index called gpu_metrics.
 *     tags:
 *       - GPU Metrics
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             properties:
 *               cpu:
 *                 type: number
 *                 description: The CPU usage of the GPU
 *               temp:
 *                 type: number
 *                 description: The temperature of the GPU
 *             required:
 *               - cpu
 *               - temp
 *     responses:
 *       200:
 *         description: Successfully posted GPU metrics
 *       400:
 *         description: Invalid input
 */
router.post('/postGpuMetrics', controller.postGpuMetrics);

/**
 * @swagger
 * /getGpuMetrics:
 *   get:
 *     summary: Get GPU metrics
 *     description: Retrieves GPU metrics (CPU, temp) from the Elasticsearch index gpu_metrics.
 *     tags:
 *       - GPU Metrics
 *     parameters:
 *       - name: startTime
 *         in: query
 *         required: false
 *         description: The start time for the metrics in ISO 8601 format (e.g., 2025-01-17T00:00:00Z).
 *         schema:
 *           type: string
 *           format: date-time
 *       - name: endTime
 *         in: query
 *         required: false
 *         description: The end time for the metrics in ISO 8601 format (e.g., 2025-01-17T23:59:59Z).
 *         schema:
 *           type: string
 *           format: date-time
 *     responses:
 *       200:
 *         description: Successfully retrieved GPU metrics
 *         content:
 *           application/json:
 *             schema:
 *               type: object
 *               properties:
 *                 success:
 *                   type: boolean
 *                   description: Indicates whether the operation was successful
 *                   example: true
 *                 metrics:
 *                   type: array
 *                   items:
 *                     type: object
 *                     properties:
 *                       timestamp:
 *                         type: string
 *                         format: date-time
 *                         description: The time the metric was recorded
 *                       cpu:
 *                         type: number
 *                         description: The CPU usage of the GPU
 *                       temp:
 *                         type: number
 *                         description: The temperature of the GPU
 *       400:
 *         description: Invalid input (e.g., incorrect date format or query parameter)
 *       404:
 *         description: No GPU metrics found for the specified criteria
 *       500:
 *         description: Internal server error
 */
router.get('/getGpuMetrics', controller.getGpuMetrics);
//TODO: Implement the following routes
// GET /function/_search

// GET /workflow/_search


module.exports = router;


//mqtt next week
//video: store only the portion of the video of tagged with an event at JETSON
