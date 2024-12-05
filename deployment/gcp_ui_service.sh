#!/bin/bash

# This script sets up the UI service on a GCP instance. It only needs to be run once on a new instance.
# The purpose of this script is to ensure that the UI service starts automatically after a reboot or instance start,
# and continues to run in the background. The UI will be managed as a systemd service and will automatically restart on reboot.
# To stop the UI service, use the following command: sudo systemctl stop nullaway-ui.service
# Service logs can be check here - sudo journalctl -u nullaway-ui.service
# Edit service conf : vi /etc/systemd/system/nullaway-ui.service


UI_PATH="/opt/NullAway-UI/ui/target/release"
LOG_DIR="/var/log/nullaway-ui/"
SERVICE_FILE="/etc/systemd/system/nullaway-ui.service"
USER=$(whoami)


echo "Creating log directory at $LOG_DIR..."
if [ ! -d "$LOG_DIR" ]; then
    echo "Creating log directory at $LOG_DIR..."
    mkdir -p $LOG_DIR
    chmod 755 $LOG_DIR
else
    echo "Log directory $LOG_DIR already exists."
fi


echo "Creating systemd service file at $SERVICE_FILE..."
cat <<EOF | sudo tee $SERVICE_FILE > /dev/null
[Unit]
Description=NullAway UI Service
After=network.target

[Service]
ExecStart=$UI_PATH/ui
WorkingDirectory=$UI_PATH
StandardOutput=append:$LOG_DIR/output.log
StandardError=append:$LOG_DIR/error.log
Restart=always
User=$USER

[Install]
WantedBy=multi-user.target
EOF


echo "Reloading systemd daemon..."
sudo systemctl daemon-reload


echo "Enabling nullaway-ui service to start on boot..."
sudo systemctl enable nullaway-ui.service


echo "Starting nullaway-ui service..."
sudo systemctl start nullaway-ui.service


echo "Checking nullaway-ui service status..."
sudo systemctl status nullaway-ui.service --no-pager

echo "Setup complete. NullAway UI is running and will start automatically on reboot."