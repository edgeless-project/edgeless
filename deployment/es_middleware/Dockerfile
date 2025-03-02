FROM node:18

# Create app directory
WORKDIR /usr/src/app

# Install app dependencies
COPY package*.json ./
RUN npm install

# Bundle app source
COPY . .


# Expose port
EXPOSE 3000

# Command to run the app
CMD ["node", "app.js"]