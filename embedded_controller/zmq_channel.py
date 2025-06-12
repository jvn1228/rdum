import zmq
import sys
from typing import Optional, Dict, Any
from google.protobuf.json_format import MessageToDict
import logging
from dataclasses import dataclass, field

logger = logging.getLogger(__name__)

@dataclass
class TrackState:
    slots: list[int] = field(default_factory=list)
    name: str = ""
    idx: int = 0
    len: int = 0
    sample_path: str = ""

    def __post_init__(self):
        self.idx = int(self.idx)
        self.len = int(self.len)

@dataclass
class State:
    tempo: int = 120
    trks: list[TrackState] = field(default_factory=list)
    division: int = 8
    default_len: int = 8
    latency: float = 0.0
    playing: bool = False
    pattern_id: int = 0
    pattern_len: int = 8
    pattern_name: str = ""
    queued_pattern_id: int = 0
    swing: int = 0

    def __post_init__(self):
        self.trks = [TrackState(**trk) for trk in self.trks]
        self.division = int(self.division)
        self.default_len = int(self.default_len)
        self.pattern_len = int(self.pattern_len)
        self.queued_pattern_id = int(self.queued_pattern_id)
        self.swing = int(self.swing)

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

            return State(**state_dict)
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