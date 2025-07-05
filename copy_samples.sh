#!/bin/bash

# Check if IP address is provided
if [ $# -ne 1 ]; then
    echo "Usage: $0 <ip_address>"
    echo "Example: $0 192.168.68.83"
    exit 1
fi

IP_ADDRESS=$1

# Run scp command to copy samples folder
echo "Copying samples folder to admin@${IP_ADDRESS}:~/dev/rdum/"
scp -r ./samples admin@${IP_ADDRESS}:~/dev/rdum/

# Check if the copy was successful
if [ $? -eq 0 ]; then
    echo "Successfully copied samples folder to admin@${IP_ADDRESS}:~/dev/rdum/"
else
    echo "Failed to copy samples folder"
    exit 1
fi
