import zmq
import sys
from typing import Optional, Dict, Any
from google.protobuf.json_format import MessageToDict
import logging
from dataclasses import dataclass, field

logger = logging.getLogger(__name__)

@dataclass
class TrackState:
    slots: list[int]
    name: str

@dataclass
class State:
    tempo: int = 120
    trk_idx: int = 0
    trks: list[TrackState] = field(default_factory=list)
    division: int = 4
    len: int = 16
    latency: float = 0.0
    playing: bool = False

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
    
    def close(self):
        """Close the ZMQ socket and context"""
        logger.info("Closing ZMQ connection")
        self.socket.close()
        self.context.term()