#!/bin/bash

# This script is designed to build and deploy code changes on a GCP instance.
# It automates pulling changes from the Git main branch, building the changes, creating a new release,
# and restarting the UI service to use the new release.


handle_error() {
    echo "Error: Command failed: $1"
    exit 1
}

run_command() {
    echo "$2"
    eval "$1" || handle_error "$1"
}

run_command "cd /opt/NullAway-UI/"
run_command "git pull origin main" "Pulling the latest changes from the main branch..."
run_command "cd /opt/NullAway-UI/ui/frontend"
run_command "yarn build" "Building the frontend with Yarn..."
run_command "cd /opt/NullAway-UI/ui"
run_command "cargo build --release" "Building the UI with Cargo in release mode..."
run_command "cd /opt/NullAway-UI/compiler"
run_command "./build.sh" "Executing the compiler build script..."
run_command "sudo systemctl restart nullaway-ui.service" "Restarting the nullaway-ui service..."

SERVICE_STATUS=$(sudo systemctl is-active nullaway-ui.service)

if [ "$SERVICE_STATUS" == "active" ]; then
    echo "Service restarted successfully."
else
    echo "Service failed to restart."
    exit 1
fi
