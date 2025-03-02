FROM node:18

# Install Git (some Node.js images might not have it)
RUN apt update && apt install -y git

# Set the working directory
WORKDIR /app

# Clone the repository (modify for private repos)
ARG GIT_REPO="https://github.com/edgeless-project/ES-middleware.git"
RUN git clone ${GIT_REPO} repo

# Set working directory to the cloned repo
WORKDIR /app/repo

# Install dependencies
RUN npm install

RUN npm run build

# Expose the port your app listens on
EXPOSE 3000

# Set the command to start the service
CMD ["node", "app.js"]