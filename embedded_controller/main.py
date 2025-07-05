from dataclasses import dataclass, field
from PIL import Image, ImageDraw, ImageTk
import time
import logging
import sys # Added import for sys
from zmq_channel import ZMQChannel, State
import modules
from modules import UIState
from hardware import IS_LINUX_OS, Pad, Pi5Pixelbuf, Switch, MockDigitalInOut, MockIncrementalEncoder
if IS_LINUX_OS:
    import board
    import digitalio
    import adafruit_ssd1306
    from adafruit_blinka.microcontroller.bcm283x.rotaryio import IncrementalEncoder
    import adafruit_mcp3xxx.mcp3008 as MCP
    from adafruit_mcp3xxx.analog_in import AnalogIn
    import busio
else:
    # Mock imports for non-Linux
    import tkinter as tk
    class MockSSD1306:
        _instances = []
        _root = None # Static variable for the root Tkinter instance

        def __init__(self, width, height, i2c, addr, reset):
            self.width = width
            self.height = height
            self._image = Image.new("1", (self.width, self.height))
            self._tk_image = None
            self._window = None
            self._canvas = None
            MockSSD1306._instances.append(self)

            if MockSSD1306._root is None:
                MockSSD1306._root = tk.Tk()
                MockSSD1306._root.withdraw() # Hide the main root window
                MockSSD1306._root.protocol("WM_DELETE_WINDOW", self._on_root_closing) # Handle closing of the hidden root

            self._create_window()

        def _create_window(self):
            # Use Toplevel associated with the root
            self._window = tk.Toplevel(MockSSD1306._root)
            self._window.title(f"Mock SSD1306 Display ({self.width}x{self.height})")
            self._window.protocol("WM_DELETE_WINDOW", self._on_closing)
            self._canvas = tk.Canvas(self._window, width=self.width*4, height=self.height*4, bg="black")
            self._canvas.pack()

        def _on_closing(self):
            logger.info("Closing MockSSD1306 window.")
            self._window.destroy()
            MockSSD1306._instances.remove(self)
            if not MockSSD1306._instances and MockSSD1306._root:
                # If no more MockSSD1306 windows, destroy the root Tkinter loop
                MockSSD1306._root.quit()
                MockSSD1306._root = None # Clear reference

        def _on_root_closing(self):
            # This handles if the hidden root window is somehow closed (e.g., by a system-wide close all)
            # It should destroy all Toplevel windows and then quit the root.
            for instance in list(MockSSD1306._instances): # Iterate over a copy to allow modification
                instance._window.destroy()
            MockSSD1306._instances.clear()
            if MockSSD1306._root:
                MockSSD1306._root.quit()
                MockSSD1306._root = None

        def fill(self, color):
            self._image = Image.new("1", (self.width*4, self.height*4), color)

        def image(self, img):
            self._image = img

        def show(self):
            if self._window and self._canvas:
                self._tk_image = ImageTk.PhotoImage(self._image.resize((self.width * 4, self.height * 4), Image.NEAREST))
                self._canvas.create_image(0, 0, anchor=tk.NW, image=self._tk_image)
                self._window.update_idletasks()
                self._window.update()
                # Also update the root to ensure all events are processed
                if MockSSD1306._root:
                    MockSSD1306._root.update_idletasks()
                    MockSSD1306._root.update()
    adafruit_ssd1306 = MockSSD1306

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[logging.StreamHandler()]
)
logger = logging.getLogger(__name__)

# Change these
# to the right size for your display!
WIDTH: int = 128
HEIGHT: int = 64
REFRESH: float = 1/30

if IS_LINUX_OS:
    # Define the Reset Pin
    oled_reset = digitalio.DigitalInOut(board.D4)

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

    switch = digitalio.DigitalInOut(board.D6)
    switch.switch_to_input(pull=digitalio.Pull.UP)

    # adc
    spi = busio.SPI(clock=board.SCK, MISO=board.MISO, MOSI=board.MOSI)
    cs = digitalio.DigitalInOut(board.D5)
    mcp = MCP.MCP3008(spi, cs)

    # neopixel strip
    pixels = Pi5Pixelbuf(board.D12, 8, auto_write=True, byteorder="BGR")
    pixels.fill(0)
    pixels.show()
else:
    # Initialize mock hardware components for non-Linux
    oled1 = MockSSD1306(WIDTH, HEIGHT, None, addr=0x3C, reset=None)
    oled2 = MockSSD1306(WIDTH, HEIGHT, None, addr=0x3D, reset=None)
    encoder_main = MockIncrementalEncoder(None, None)
    button_main = MockDigitalInOut(None)
    encoder_sub = MockIncrementalEncoder(None, None)
    button_sub = MockDigitalInOut(None)
    switch = MockDigitalInOut(None)
    mcp = None # MCP is not used directly in the mock setup
    pixels = Pi5Pixelbuf(None, 8, auto_write=True, byteorder="BGR")

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

        if IS_LINUX_OS:
            self._enc1 = encoder_main
            self._enc2 = encoder_sub
            self._button1 = button_main
            self._button2 = button_sub
            self._pads = [Pad(AnalogIn(mcp, MCP.P0)), Pad(AnalogIn(mcp, MCP.P1)), Pad(AnalogIn(mcp, MCP.P2)), Pad(AnalogIn(mcp, MCP.P3)),
                Pad(AnalogIn(mcp, MCP.P4)), Pad(AnalogIn(mcp, MCP.P5)), Pad(AnalogIn(mcp, MCP.P6)), Pad(AnalogIn(mcp, MCP.P7))]
            self._switch = Switch(switch)
        else:
            from keyboard_input import KeyboardInputManager
            self._keyboard_manager = KeyboardInputManager()
            self._enc1 = self._keyboard_manager.main_encoder
            self._enc2 = self._keyboard_manager.sub_encoder
            self._button1 = self._keyboard_manager.main_button
            self._button2 = self._keyboard_manager.sub_button
            self._pads = self._keyboard_manager.pads
            self._switch = self._keyboard_manager.switch

        # Create blank image for drawing.
        # Make sure to create image with mode '1' for 1-bit color.
        self._image1 = Image.new("1", (self._oled1.width, self._oled1.height))
        self._draw1 = ImageDraw.Draw(self._image1)

        self._image2 = Image.new("1", (self._oled2.width, self._oled2.height))
        self._draw2 = ImageDraw.Draw(self._image2)

        self._pixels = pixels

        self._last_state = State()
        self._last_refresh: float = time.monotonic()

        # Create and connect the state receiver/command sender
        self._channel = ZMQChannel("tcp://localhost:5555")
        if not self._channel.connect():
            sys.exit(1)

        # current module activated
        self._input_state = UIState()
        self._input_state.switch = self._switch
        self._modules: list[Module] = [
            modules.Status(self._channel, self._input_state),
            modules.Playback(self._channel, self._input_state)
        ]
        self._module_idx = 0

    def receive_state(self):
        self._modules[self._module_idx].receive_state()

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
        self._input_state.pad_armed = [pad.armed for pad in self._pads]
        self._modules[self._module_idx].on_input_update()

    def render(self):
        self._draw1.rectangle((0, 0, self._oled1.width, self._oled1.height), fill=0)
        self._draw2.rectangle((0, 0, self._oled2.width, self._oled2.height), fill=0)
        # self._pixels.fill(0)

        self._modules[self._module_idx].render_primary(self._draw1)
        self._modules[self._module_idx].render_secondary(self._draw2)
        self._modules[self._module_idx].render_leds(self._pixels)

        self._oled1.image(self._image1)
        self._oled2.image(self._image2)
        self._oled1.show()
        self._oled2.show()
        self._pixels.show()

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
        pixels.fill(0)
        pixels.show()
        if not IS_LINUX_OS:
            controller._keyboard_manager.stop()
        # controller._channel.close()
        logger.info("Display cleared")