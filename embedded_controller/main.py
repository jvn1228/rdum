from codecs import IncrementalEncoder
from dataclasses import dataclass, field
import board
import digitalio
from PIL import Image, ImageDraw
import time
import logging
from zmq_channel import ZMQChannel, State
import adafruit_ssd1306
from adafruit_blinka.microcontroller.bcm283x.rotaryio import IncrementalEncoder
import adafruit_mcp3xxx.mcp3008 as MCP
from adafruit_mcp3xxx.analog_in import AnalogIn
import busio
import modules
from modules import UIState

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[logging.StreamHandler()]
)
logger = logging.getLogger(__name__)

# Define the Reset Pin
oled_reset = digitalio.DigitalInOut(board.D4)

# Change these
# to the right size for your display!
WIDTH: int = 128
HEIGHT: int = 64
REFRESH: float = 1/30


BORDER: int = 5

# Use for I2C.
i2c = board.I2C()  # uses board.SCL and board.SDA
# i2c = board.STEMMA_I2C()  # For using the built-in STEMMA QT connector on a microcontroller
oled1 = adafruit_ssd1306.SSD1306_I2C(WIDTH, HEIGHT, i2c, addr=0x3C, reset=oled_reset)
oled2 = adafruit_ssd1306.SSD1306_I2C(WIDTH, HEIGHT, i2c, addr=0x3D, reset=oled_reset)

#encoders
encoder_main = IncrementalEncoder(board.D17, board.D18)
button_main = digitalio.DigitalInOut(board.D15)
button_main.switch_to_input(pull=digitalio.Pull.UP)

encoder_sub = IncrementalEncoder(board.D22, board.D23)
button_sub = digitalio.DigitalInOut(board.D27)
button_sub.switch_to_input(pull=digitalio.Pull.UP)

# adc
spi = busio.SPI(clock=board.SCK, MISO=board.MISO, MOSI=board.MOSI)
cs = digitalio.DigitalInOut(board.D5)
mcp = MCP.MCP3008(spi, cs)

class Pad:
    """
        Pad specific wrapper for adc channel.
        When initialized, reads the value and uses that as
        the zero point.
        Value is floored at 0, even if it would be negative
        with the offset.
    """
    def __init__(self, adc: AnalogIn):
        self._adc = adc
        self._offset = adc.value
        # the adc library returns a 16-bit integer even though
        # precision of the adc is only 10 bits
        self._max = 2**16 - self._offset
    
    @property
    def value(self) -> int:
        return min(max(0, self._adc.value - self._offset), self._max)
    
    @property
    def percent(self) -> int:
        return self.value // self._max

    def zero(self):
        self._offset = self._adc.value

class EmbeddedController:
    """
    Controller for embedded devices. Handles display and input.
    Receives state updates via ZeroMQ. The main drum sequencer is
    implemented in Rust.
    """
    def __init__(self):
        # hardware interfaces
        self._oled1 = oled1
        self._oled2 = oled2

        self._enc1 = encoder_main
        self._enc2 = encoder_sub
        self._button1 = button_main
        self._button2 = button_sub

        self._pads = [Pad(AnalogIn(mcp, MCP.P0)), Pad(AnalogIn(mcp, MCP.P1)), Pad(AnalogIn(mcp, MCP.P2)), Pad(AnalogIn(mcp, MCP.P3)),
            Pad(AnalogIn(mcp, MCP.P4)), Pad(AnalogIn(mcp, MCP.P5)), Pad(AnalogIn(mcp, MCP.P6)), Pad(AnalogIn(mcp, MCP.P7))]

        # Create blank image for drawing.
        # Make sure to create image with mode '1' for 1-bit color.
        self._image1 = Image.new("1", (self._oled1.width, self._oled1.height))
        self._draw1 = ImageDraw.Draw(self._image1)

        self._image2 = Image.new("1", (self._oled2.width, self._oled2.height))
        self._draw2 = ImageDraw.Draw(self._image2)

        self._last_state = State()
        self._last_refresh: float = time.monotonic()

        # current module activated
        self._input_state = UIState()
        self._modules: list[Module] = [modules.Status(self._input_state), modules.Playback()]
        self._module_idx = 1

        # Create and connect the state receiver
        self._channel = ZMQChannel("tcp://192.168.68.73:5555")
        if not self._channel.connect():
            sys.exit(1)

    def receive_state(self):
        self._modules[self._module_idx].on_state_update(self._channel.receive_state())

    def receive_input(self):
        self._input_state.enc1_d = self._enc1.position - self._input_state.enc1_pos
        self._input_state.enc1_pos = self._enc1.position
        
        if self._input_state.enc1_d > 0:
            self._module_idx = (self._module_idx + 1) % len(self._modules)
        elif self._input_state.enc1_d < 0:
            self._module_idx = (self._module_idx - 1) % len(self._modules)

        self._input_state.enc2_d = self._enc2.position - self._input_state.enc2_pos
        self._input_state.enc2_pos = self._enc2.position
        
        self._input_state.button1_pressed = not self._button1.value
        self._input_state.button2_pressed = not self._button2.value
        self._input_state.pad_values = [pad.value for pad in self._pads]
        self._modules[self._module_idx].on_input_update()

    def render(self):
        self._draw1.rectangle((0, 0, self._oled1.width, self._oled1.height), fill=0)
        self._draw2.rectangle((0, 0, self._oled2.width, self._oled2.height), fill=0)

        self._modules[self._module_idx].render_primary(self._draw1)
        self._modules[self._module_idx].render_secondary(self._draw2)

        self._oled1.image(self._image1)
        self._oled2.image(self._image2)
        self._oled1.show()
        self._oled2.show()

    def run(self):
        while True:
            self.receive_input()
            if time.monotonic() - self._last_refresh > REFRESH:
                self._last_state = self.receive_state()
                self.render()
                self._last_refresh = time.monotonic()
            
if __name__ == "__main__":
    controller = EmbeddedController()
    try:
        controller.run()
    except KeyboardInterrupt:
        logger.info("Exiting and clearing display...")
        # Clear the display
        controller._oled1.fill(0)
        controller._oled1.show()
        controller._oled2.fill(0)
        controller._oled2.show()
        # controller._channel.close()
        logger.info("Display cleared")