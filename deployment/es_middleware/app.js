const express = require('express');
const bodyParser = require('body-parser');
const { Client } = require('@elastic/elasticsearch');
const routes = require('./routes/apiRoutes');
const controller = require('./controller/apiController');
const swagger = require('./swagger');
const cors = require('cors');
const mqtt = require('mqtt');

const port = 3000;
const app = express();

// Set up the Elasticsearch client
const client = new Client({
    node: process.env.ELASTICSEARCH_HOST,
    auth: {
        username: process.env.ELASTICSEARCH_USERNAME,
        password: process.env.ELASTICSEARCH_PASSWORD
    }
});
// Pass the Elasticsearch client to the routes
app.use((req, res, next) => {
    req.client = client;
    next();
});

// Initialize Middleware
app.use(bodyParser.json());
app.use(cors());
app.use(routes);
swagger(app);


// Start the HTTP server
const server = app.listen(port, () => {
    console.log(`Server running at http://localhost:${port}/`);
});

// Initialize Socket.io
const io = require('socket.io')(server);

io.on('connection', (socket) => {
    console.log('USER CONNECTED');

    socket.on('disconnect', () => {
        console.log('USER DISCONNECTED');
    });
});

// MQTT Configuration
function initMqttSubscriber() {
    const options = {
        host: "mqtt.ubiwhere.com",
        port: 8883,
        username: "playground",
        password: "xYV49Y09rdMp",
        protocol: 'mqtts', // Use 'mqtts' for TLS
        rejectUnauthorized: true
    };
    console.log("mqtt running");

    // Connect to the MQTT broker with TLS
    const mqttClient = mqtt.connect(options);

    mqttClient.on('connect', () => {
        console.log('Connected to MQTT broker with TLS');

        // Subscribe to the topics
        const topics = ['edgeless-uw-uc-counters', 'edgeless-uw-uc-metrics'];
        mqttClient.subscribe(topics, (err) => {
            if (!err) {
                console.log(`Subscribed to topics: ${topics.join(', ')}`);
            } else {
                console.error('Subscription error:', err);
            }
        });
    });

    // Listen for messages on subscribed topics
    mqttClient.on('message', (topic, message) => {
        console.log(`Received message on topic ${topic}: ${message.toString()}`);

        // Forward the message to all connected Socket.io clients
        if (topic === 'edgeless-uw-uc-counters') {
            io.of('/live').emit('counter', {
                topic,
                data: JSON.parse(message.toString())
            });
        } else if (topic === 'edgeless-uw-uc-metrics') {
            io.of('/live').emit('metric', {
                topic,
                data: JSON.parse(message.toString())
            });
        }
    });

    // Emit random counter data every 4 seconds
    setInterval(() => {
        const randomData = {
            cam_id: Math.floor(Math.random() * 3) + 1, // Random number between 1 and 3
            counters: {
                car: Math.floor(Math.random() * 30) + 1, // Random number between 1 and 30
                person: Math.floor(Math.random() * 30) + 1, // Random number between 1 and 30
                truck: Math.floor(Math.random() * 30) + 1, // Random number between 1 and 30
                bicycle: Math.floor(Math.random() * 30) + 1, // Random number between 1 and 30
                motorbike: Math.floor(Math.random() * 30) + 1, // Random number between 1 and 30
                bus: Math.floor(Math.random() * 30) + 1, // Random number between 1 and 30
                crash: Math.floor(Math.random() * 6) // Random number between 0 and 5
            },
            timestamp: new Date().toISOString() // Current timestamp in ISO format
        };

        io.of('/live').emit('counter', randomData);
        console.log('counter randomData:', randomData);
    }, 5000);

    // Emit random metric data every 4 seconds
    setInterval(() => {
        const randomMetricData = {
            cam_id: Math.floor(Math.random() * 3) + 1, // Random cam_id between 0 and 2
            tracking_speed: (Math.random() * 0.005 + 0.001).toFixed(3), // Random between 0.001 and 0.006
            inference_speed: (Math.random() * 0.005 + 0.001).toFixed(3), // Random between 0.001 and 0.006
            system_cpu_usage: (Math.random() * 100).toFixed(1), // Random percentage 0.0 to 100.0
            system_memory_usage: (Math.random() * 100).toFixed(1), // Random percentage 0.0 to 100.0
            system_memory_available: (Math.random() * 100).toFixed(1), // Random percentage 0.0 to 100.0
            jetson_gpu_usage: (Math.random() * 100).toFixed(1), // Random percentage 0.0 to 100.0
            timestamp: new Date().toISOString() // Current timestamp in ISO format
        };

        io.of('/live').emit('metric', randomMetricData);
        console.log('metric randomData:', randomMetricData);
    }, 5000);
}

// Initialize MQTT Subscriber
// initMqttSubscriber();

// Test Elasticsearch connection on server start
(async () => {
    await controller.testElasticsearchConnection(client);
})();


