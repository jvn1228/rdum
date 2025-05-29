#!/usr/bin/env python3
"""
ZMQ Test Receiver for RDUM

This script connects to the RDUM ZeroMQ server, receives messages,
decodes the protobuf data, and logs the decoded data.
"""

import zmq
import logging
import time
import os
import sys
import subprocess
from typing import Optional, Dict, Any
import argparse
from google.protobuf.json_format import MessageToDict

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[logging.StreamHandler()]
)
logger = logging.getLogger(__name__)

# Ensure the protobuf module is generated
def ensure_protobuf_module():
    """Generate Python protobuf modules if they don't exist"""
    # Path to the proto directory
    proto_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "proto")
    state_proto = os.path.join(proto_dir, "state.proto")
    
    # Output directory for generated Python modules
    output_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "proto_gen")
    os.makedirs(output_dir, exist_ok=True)
    
    # Check if the module already exists
    if not os.path.exists(os.path.join(output_dir, "state_pb2.py")):
        logger.info("Generating protobuf Python modules...")
        cmd = [
            "protoc",
            f"--proto_path={proto_dir}",
            f"--python_out={output_dir}",
            state_proto
        ]
        try:
            subprocess.run(cmd, check=True)
            logger.info("Successfully generated protobuf modules")
            
            # Create __init__.py file to make it a proper Python package
            with open(os.path.join(output_dir, "__init__.py"), "w") as f:
                pass
                
        except subprocess.CalledProcessError as e:
            logger.error(f"Failed to generate protobuf modules: {e}")
            logger.error("Make sure 'protoc' is installed. Install with: brew install protobuf")
            sys.exit(1)
    
    # Add the directory to Python path so we can import the generated modules
    sys.path.append(os.path.dirname(os.path.abspath(__file__)))


class ZMQReceiver:
    """Receives and decodes ZMQ messages from RDUM"""
    
    def __init__(self, server_address: str = "tcp://localhost:5555"):
        self.server_address = server_address
        self.context = zmq.Context()
        self.socket = self.context.socket(zmq.REQ)  # REQ socket to pair with the REP socket in the server
        
        # Import the generated protobuf modules
        try:
            from proto_gen import state_pb2
            self.state_pb2 = state_pb2
        except ImportError:
            logger.error("Could not import protobuf modules. Make sure they were generated correctly.")
            sys.exit(1)
    
    def connect(self):
        """Connect to the ZMQ server"""
        logger.info(f"Connecting to ZMQ server at {self.server_address}")
        try:
            self.socket.connect(self.server_address)
            logger.info("Connected successfully")
            return True
        except zmq.ZMQError as e:
            logger.error(f"Failed to connect: {e}")
            return False
    
    def receive_state(self) -> Optional[Dict[str, Any]]:
        """Send an empty message to trigger a response, then receive and decode the state"""
        try:
            # Send an empty message to trigger a response
            self.socket.send(b'')
            
            # Receive the response
            message = self.socket.recv()
            
            # Decode the protobuf message
            state = self.state_pb2.State()
            state.ParseFromString(message)
            
            # Convert to dictionary for easier logging
            state_dict = MessageToDict(
                state,
                preserving_proto_field_name=True
            )
            
            return state_dict
        except zmq.ZMQError as e:
            logger.error(f"ZMQ error: {e}")
            return None
        except Exception as e:
            logger.error(f"Error receiving state: {e}")
            return None
    
    def close(self):
        """Close the ZMQ socket and context"""
        logger.info("Closing ZMQ connection")
        self.socket.close()
        self.context.term()


def main():
    """Main function to run the ZMQ receiver"""
    parser = argparse.ArgumentParser(description="ZMQ Test Receiver for RDUM")
    parser.add_argument(
        "--server", 
        default="tcp://localhost:5555",
        help="ZMQ server address (default: tcp://localhost:5555)"
    )
    parser.add_argument(
        "--interval", 
        type=float, 
        default=0.5,
        help="Polling interval in seconds (default: 0.5)"
    )
    args = parser.parse_args()
    
    # Ensure protobuf modules are generated
    ensure_protobuf_module()
    
    # Create and connect the receiver
    receiver = ZMQReceiver(args.server)
    if not receiver.connect():
        sys.exit(1)
    
    try:
        logger.info(f"Starting to receive messages every {args.interval} seconds. Press Ctrl+C to stop.")
        
        while True:
            state = receiver.receive_state()
            if state:
                logger.info(f"Received state: {state}")
            time.sleep(args.interval)
    
    except KeyboardInterrupt:
        logger.info("Receiver stopped by user")
    except Exception as e:
        logger.error(f"Unexpected error: {e}")
    finally:
        receiver.close()


if __name__ == "__main__":
    main()
