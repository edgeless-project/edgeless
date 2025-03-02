let io;

module.exports = {
    init: httpServer => {
        io = require('socket.io')(httpServer, {
            reconnection: true, // Enable reconnection
            reconnectionAttempts: 10, // Number of reconnection attempts
            reconnectionDelay: 1000, // Initial delay between reconnection attempts (milliseconds)
            reconnectionDelayMax: 5000, // Maximum delay between reconnection attempts (milliseconds)
            randomizationFactor: 0.5, // Randomization factor for reconnection attempts
            cors: {
              origin: '*', // Allow requests from any origin (for dev purposes)
            }
        });
        return io;
    },
    getIO: () => {
        if (!io){
            return new Error('Socket.io not initialized');
        }
        return io;
    }
}