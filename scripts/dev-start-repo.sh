#!/usr/bin/env bash

set -e # Exit immediately if a command exits with a non-zero status.

# Default configuration
DEFAULT_REPO_DIR="/tmp/avocado-dev-repo"
DEFAULT_PORT="8080"
DEFAULT_CONTAINER_NAME="avocado-dev-repo"
DEFAULT_NETWORK_NAME="avocado-dev-network"

# Function to show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Start a development package repository server using Docker.

Options:
    -r, --repo-dir DIR      Repository directory (default: $DEFAULT_REPO_DIR)
    -p, --port PORT         Host port to bind to (default: $DEFAULT_PORT)
    -n, --name NAME         Container name (default: $DEFAULT_CONTAINER_NAME)
    --network NETWORK       Docker network name (default: $DEFAULT_NETWORK_NAME)
    --stop                  Stop the running repository server
    --restart               Restart the repository server
    --logs                  Show container logs
    --status                Show container status
    -h, --help              Show this help message

Examples:
    $0                                          # Start with defaults
    $0 -r /opt/avocado-repo -p 9000            # Custom repo dir and port
    $0 --stop                                   # Stop the server
    $0 --restart                               # Restart the server
    $0 --logs                                   # View logs
    $0 --status                                 # Check status

The repository server will serve packages from the specified directory and
automatically update metadata when packages are added or changed.

Repository structure expected:
    <repo-dir>/
    ├── packages/<distro-codename>/     # Package files
    └── releases/<distro-codename>/     # Repository metadata

The server will be accessible at http://localhost:<port>/

EOF
}

# Function to check if container exists
container_exists() {
    docker ps -a --filter "name=^${CONTAINER_NAME}$" --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"
}

# Function to check if container is running
container_running() {
    docker ps --filter "name=^${CONTAINER_NAME}$" --filter "status=running" --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"
}

# Function to check if network exists
network_exists() {
    docker network ls --filter "name=^${NETWORK_NAME}$" --format "{{.Name}}" | grep -q "^${NETWORK_NAME}$"
}

# Function to create network if it doesn't exist
ensure_network() {
    if ! network_exists; then
        echo "Creating Docker network: $NETWORK_NAME"
        docker network create "$NETWORK_NAME"
    else
        echo "Using existing Docker network: $NETWORK_NAME"
    fi
}

# Function to build the package-repo image if it doesn't exist
ensure_image() {
    if ! docker images --format "{{.Repository}}:{{.Tag}}" | grep -q "^avocadolinux/package-repo:local$"; then
        echo "Building package-repo Docker image..."
        docker build -t avocadolinux/package-repo:local distro/support/package-repo/
    else
        echo "Using existing package-repo Docker image"
    fi
}

# Function to stop container
stop_container() {
    if container_running; then
        echo "Stopping container: $CONTAINER_NAME"
        docker stop "$CONTAINER_NAME"
    fi
    
    if container_exists; then
        echo "Removing container: $CONTAINER_NAME"
        docker rm "$CONTAINER_NAME"
    fi
}

# Function to start container
start_container() {
    # Validate repository directory
    if [ ! -d "$REPO_DIR" ]; then
        echo "Error: Repository directory '$REPO_DIR' not found" >&2
        echo "Create it first or use dev-sync-packages.sh to populate it." >&2
        exit 1
    fi
    
    # Ensure network and image exist
    ensure_network
    ensure_image
    
    # Stop existing container if running
    if container_running; then
        echo "Container $CONTAINER_NAME is already running"
        echo "Use --restart to restart it or --stop to stop it"
        return 0
    fi
    
    # Remove existing stopped container
    if container_exists; then
        echo "Removing existing stopped container: $CONTAINER_NAME"
        docker rm "$CONTAINER_NAME"
    fi
    
    echo "Starting package repository server..."
    echo "Repository directory: $REPO_DIR"
    echo "Container name: $CONTAINER_NAME"
    echo "Network: $NETWORK_NAME"
    echo "Port: $PORT"
    
    # Start the container
    docker run -d \
        --name "$CONTAINER_NAME" \
        --network "$NETWORK_NAME" \
        -p "$PORT:80" \
        -e USER_ID="$(id -u)" \
        -e GROUP_ID="$(id -g)" \
        -v "$REPO_DIR:/repo" \
        avocadolinux/package-repo:local
    
    echo "✓ Container started successfully"
    
    # Wait for nginx to start
    echo "Waiting for nginx to start..."
    sleep 3
    
    # Verify container is still running
    if ! container_running; then
        echo "✗ Container failed to start or exited unexpectedly" >&2
        echo "Container logs:" >&2
        docker logs "$CONTAINER_NAME" >&2
        exit 1
    fi
    
    echo "✓ Package repository server is running"
    echo ""
    echo "Repository URL: http://localhost:$PORT/"
    echo "Container name: $CONTAINER_NAME"
    echo "Network name: $NETWORK_NAME"
    echo ""
    echo "Use 'docker logs $CONTAINER_NAME' to view logs"
    echo "Use '$0 --stop' to stop the server"
}

# Function to show container status
show_status() {
    if container_running; then
        echo "✓ Container $CONTAINER_NAME is running"
        docker ps --filter "name=^${CONTAINER_NAME}$" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
        echo ""
        echo "Repository URL: http://localhost:$PORT/"
    elif container_exists; then
        echo "⚠ Container $CONTAINER_NAME exists but is not running"
        docker ps -a --filter "name=^${CONTAINER_NAME}$" --format "table {{.Names}}\t{{.Status}}"
    else
        echo "✗ Container $CONTAINER_NAME does not exist"
    fi
}

# Function to show logs
show_logs() {
    if container_exists; then
        docker logs -f "$CONTAINER_NAME"
    else
        echo "Error: Container $CONTAINER_NAME does not exist" >&2
        exit 1
    fi
}

# Parse command line arguments
REPO_DIR="$DEFAULT_REPO_DIR"
PORT="$DEFAULT_PORT"
CONTAINER_NAME="$DEFAULT_CONTAINER_NAME"
NETWORK_NAME="$DEFAULT_NETWORK_NAME"
ACTION="start"

while [[ $# -gt 0 ]]; do
    case $1 in
        -r|--repo-dir)
            REPO_DIR="$2"
            shift 2
            ;;
        -p|--port)
            PORT="$2"
            shift 2
            ;;
        -n|--name)
            CONTAINER_NAME="$2"
            shift 2
            ;;
        --network)
            NETWORK_NAME="$2"
            shift 2
            ;;
        --stop)
            ACTION="stop"
            shift
            ;;
        --restart)
            ACTION="restart"
            shift
            ;;
        --logs)
            ACTION="logs"
            shift
            ;;
        --status)
            ACTION="status"
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        -*)
            echo "Error: Unknown option $1" >&2
            usage >&2
            exit 1
            ;;
        *)
            echo "Error: Unexpected argument $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

# Convert relative path to absolute path
REPO_DIR="$(realpath "$REPO_DIR")"

echo "=== Avocado Development Repository Server ==="

case $ACTION in
    start)
        start_container
        ;;
    stop)
        stop_container
        echo "✓ Repository server stopped"
        ;;
    restart)
        stop_container
        start_container
        ;;
    logs)
        show_logs
        ;;
    status)
        show_status
        ;;
    *)
        echo "Error: Unknown action $ACTION" >&2
        exit 1
        ;;
esac
