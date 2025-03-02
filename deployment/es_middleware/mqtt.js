const mqtt = require('mqtt');
const http = require('http');
const server = require('./app').server;
// const socketIo = require('socket.io');
function initMqttSubscriber(){
    // MQTT configuration
    const options = {
        host: "mqtt.ubiwhere.com",
        port: 8883, 
        username: "playground",
        password: "xYV49Y09rdMp",
        protocol: 'mqtts', // Use 'mqtts' for TLS
        rejectUnauthorized: true 
    };

    // Connect to the MQTT broker with TLS
    const client = mqtt.connect(options);

    client.on('connect', () => {
        console.log('Connected to MQTT broker with TLS');

        // Subscribe to the topics
        const topics = ['edgeless-uw-uc-counters', 'edgeless-uw-uc-metrics']; 
        client.subscribe(topics, (err) => {
            if (!err) {
                console.log(`Subscribed to topics: ${topics.join(', ')}`);
            } else {
                console.error('Subscription error:', err);
            }
        });
    });
    // const io = require('./socket');
    // const io = require('./socket').init(server);

    // io.on('connection', (socket) => {
    //     console.log('USER CONNECTED');
    
    //     socket.on('disconnect', function () {
    //         console.log('USER DISCONNECTED');
    //     });
    // })
    // const test = 5;
    // // Listen for messages on subscribed topics
    // // Handle incoming MQTT messages and forward them via socket.io
    // client.on('edgeless', (topic, message) => {
    //     console.log(`Received message on topic ${topic}: ${message.toString()}`);
        
    //     // Send to all connected socket.io clients
    //     io.emit('mqtt_message', {
    //         topic,
    //         data: JSON.parse(message.toString())
    //     });
    // });
    // Emit a test value every 2 seconds via socket.io
// setInterval(() => {
//     const testValue = {
//         topic: 'test_topic',
//         data: { message: 'This is a test message' }
//     };

//     // Send the test value to all connected socket.io clients
//     io.of('/live').emit('mqtt_message', testValue);
//     console.log(`Sent test message to front-end: ${JSON.stringify(testValue)}`);
// }, 2000);
};


module.exports = initMqttSubscriber;