#!/usr/bin/env python3
"""
ZMQ Test Channel for RDUM

This script connects to the RDUM ZeroMQ server, allowing you to:
1. Receive and decode state messages
2. Send commands to the sequencer via command line interface
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


class ZMQChannel:
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
    
    def send_command(self, command_type, **kwargs):
        """Send a command to the ZMQ server
        
        Args:
            command_type: The Command enum value to send
            **kwargs: Command-specific arguments
        
        Returns:
            bool: True if the command was sent successfully, False otherwise
        """
        try:
            # Create the command message
            cmd_msg = self.state_pb2.CommandMessage()
            cmd_msg.command_type = command_type
            
            # Set command-specific arguments
            if command_type == self.state_pb2.COMMAND_SET_TEMPO and 'tempo' in kwargs:
                cmd_msg.tempo = kwargs['tempo']
            elif command_type == self.state_pb2.COMMAND_SET_PATTERN and 'pattern_index' in kwargs:
                cmd_msg.pattern_index = kwargs['pattern_index']
            elif command_type == self.state_pb2.COMMAND_SET_DIVISION and 'division' in kwargs:
                cmd_msg.division = kwargs['division']
            elif command_type == self.state_pb2.COMMAND_PLAY_SOUND and 'track_index' in kwargs and 'velocity' in kwargs:
                play_sound_args = self.state_pb2.PlaySoundArgs(
                    track_index=kwargs['track_index'],
                    velocity=kwargs['velocity']
                )
                cmd_msg.play_sound_args.CopyFrom(play_sound_args)
            elif command_type == self.state_pb2.COMMAND_SET_SLOT_VELOCITY and 'track_index' in kwargs and 'slot_index' in kwargs:
                slot_args = self.state_pb2.SlotArgs(
                    track_index=kwargs['track_index'],
                    slot_index=kwargs['slot_index'],
                    velocity=kwargs['velocity']
                )
                cmd_msg.slot_args.CopyFrom(slot_args)
            elif command_type == self.state_pb2.COMMAND_SET_TRACK_LENGTH and 'track_index' in kwargs and 'track_length' in kwargs:
                track_length_args = self.state_pb2.TrackLengthArgs(
                    track_index=kwargs['track_index'],
                    track_length=kwargs['track_length']
                )
                cmd_msg.track_length_args.CopyFrom(track_length_args)
            
            # Serialize the command message
            cmd_bytes = cmd_msg.SerializeToString()
            
            # Send the command
            self.socket.send(cmd_bytes)
            
            # Wait for response (needed for REQ/REP pattern)
            response = self.socket.recv()
            
            # Process response if needed (in this case, just log success)
            logger.info(f"Command sent successfully: {self.state_pb2.Command.Name(command_type)}")
            return True
        
        except Exception as e:
            logger.error(f"Error sending command: {e}")
            return False
    
    def close(self):
        """Close the ZMQ socket and context"""
        logger.info("Closing ZMQ connection")
        self.socket.close()
        self.context.term()


def main():
    """Main function to run the ZMQ channel"""
    # Ensure protobuf modules are generated
    ensure_protobuf_module()
    
    # Import protobuf modules for command names
    try:
        from proto_gen import state_pb2
    except ImportError:
        logger.error("Could not import protobuf modules. Make sure they were generated correctly.")
        sys.exit(1)
    
    # Create parser
    parser = argparse.ArgumentParser(description="ZMQ Test Channel for RDUM")
    parser.add_argument(
        "--server", 
        default="tcp://localhost:5555",
        help="ZMQ server address (default: tcp://localhost:5555)"
    )
    
    # Create subparsers for different actions
    subparsers = parser.add_subparsers(dest="action", help="Action to perform")
    
    # Monitor subcommand
    monitor_parser = subparsers.add_parser("monitor", help="Monitor state updates")
    monitor_parser.add_argument(
        "--interval", 
        type=float, 
        default=0.5,
        help="Polling interval in seconds (default: 0.5)"
    )
    
    # Play sequencer command
    subparsers.add_parser("play", help="Start the sequencer")
    
    # Stop sequencer command
    subparsers.add_parser("stop", help="Stop the sequencer")
    
    # Set tempo command
    tempo_parser = subparsers.add_parser("tempo", help="Set the tempo (BPM)")
    tempo_parser.add_argument("value", type=int, help="Tempo value in BPM (30-300)")
    
    # Set division command
    division_parser = subparsers.add_parser("division", help="Set the note division")
    division_parser.add_argument(
        "value", 
        type=int, 
        choices=[2, 3, 4, 6, 8, 12, 16, 24, 32],
        help="Division value (2=half, 4=quarter, 8=eighth, 16=sixteenth, etc.)"
    )
    
    # Set pattern command
    pattern_parser = subparsers.add_parser("pattern", help="Set the current pattern")
    pattern_parser.add_argument("index", type=int, help="Pattern index")
    
    # Play sound command
    sound_parser = subparsers.add_parser("sound", help="Play a sound from a track")
    sound_parser.add_argument("track", type=int, help="Track index")
    sound_parser.add_argument("velocity", type=int, help="Velocity (0-127)")
    
    # Set slot velocity command
    slot_parser = subparsers.add_parser("slot", help="Set velocity for a slot")
    slot_parser.add_argument("track", type=int, help="Track index")
    slot_parser.add_argument("slot", type=int, help="Slot index")
    slot_parser.add_argument("velocity", type=int, help="Velocity (0-127)")
    
    # Set track length command
    length_parser = subparsers.add_parser("length", help="Set track length")
    length_parser.add_argument("track", type=int, help="Track index")
    length_parser.add_argument("length", type=int, help="Track length (1-64)")
    
    # Parse arguments
    args = parser.parse_args()
    
    if not args.action:
        parser.print_help()
        sys.exit(1)
    
    # Create and connect the channel
    channel = ZMQChannel(args.server)
    if not channel.connect():
        sys.exit(1)
    
    try:
        if args.action == "monitor":
            logger.info(f"Starting to monitor state updates every {args.interval} seconds. Press Ctrl+C to stop.")
            while True:
                state = channel.receive_state()
                if state:
                    logger.info(f"Received state: {state}")
                time.sleep(args.interval)
                
        elif args.action == "play":
            channel.send_command(state_pb2.COMMAND_PLAY_SEQUENCER)
            
        elif args.action == "stop":
            channel.send_command(state_pb2.COMMAND_STOP_SEQUENCER)
            
        elif args.action == "tempo" and hasattr(args, "value"):
            tempo = max(30, min(300, args.value))  # Clamp tempo between 30-300 BPM
            channel.send_command(state_pb2.COMMAND_SET_TEMPO, tempo=tempo)
            
        elif args.action == "division" and hasattr(args, "value"):
            channel.send_command(state_pb2.COMMAND_SET_DIVISION, division=args.value)
            
        elif args.action == "pattern" and hasattr(args, "index"):
            channel.send_command(state_pb2.COMMAND_SET_PATTERN, pattern_index=args.index)
            
        elif args.action == "sound" and hasattr(args, "track") and hasattr(args, "velocity"):
            channel.send_command(state_pb2.COMMAND_PLAY_SOUND, 
                               track_index=args.track,
                               velocity=min(127, max(0, args.velocity)))
            
        elif args.action == "slot" and hasattr(args, "track") and hasattr(args, "slot"):
            channel.send_command(state_pb2.COMMAND_SET_SLOT_VELOCITY, 
                               track_index=args.track,
                               slot_index=args.slot,
                               velocity=min(127, max(0, args.velocity)))
            
        elif args.action == "length" and hasattr(args, "track") and hasattr(args, "length"):
            length = max(1, min(64, args.length))  # Clamp length between 1-64
            channel.send_command(state_pb2.COMMAND_SET_TRACK_LENGTH, 
                               track_index=args.track,
                               track_length=length)
                                
    except KeyboardInterrupt:
        logger.info("Channel stopped by user")
    except Exception as e:
        logger.error(f"Unexpected error: {e}")
    finally:
        channel.close()


if __name__ == "__main__":
    main()
