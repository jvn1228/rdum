from dataclasses import dataclass, field
import board
import digitalio
from PIL import Image, ImageDraw, ImageFont
import time
import logging
import adafruit_ssd1306
from zmq_channel import ZMQChannel, State

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
oled = adafruit_ssd1306.SSD1306_I2C(WIDTH, HEIGHT, i2c, addr=0x3C, reset=oled_reset)

class EmbeddedController:
    """
    Controller for embedded devices. Handles display and input.
    Receives state updates via ZeroMQ. The main drum sequencer is
    implemented in Rust.
    """
    def __init__(self):
        self._oled = oled
        # Create blank image for drawing.
        # Make sure to create image with mode '1' for 1-bit color.
        self._image = Image.new("1", (self._oled.width, self._oled.height))
        self._draw = ImageDraw.Draw(self._image)
        self._font = ImageFont.load_default()
        self._last_state = State()
        self._last_refresh: float = time.monotonic()
        # Create and connect the state receiver
        self._channel = ZMQChannel()
        if not self._channel.connect():
            sys.exit(1)

    def receive_state(self) -> State:
       return self._channel.receive_state()

    def receive_input(self):
        pass

    def render(self):
        # Display track_idx in header
        header_text = f"Step: {self._last_state.trk_idx}"
        bbox: list[int] = self._font.getbbox(header_text)
        text_width, text_height = bbox[2] - bbox[0], bbox[3] - bbox[1]
        self._draw.text(
            (self._oled.width // 2 - text_width // 2, 5),
            header_text,
            font=self._font,
            fill=255
        )
        
        # Draw a separator line
        self._draw.line([(0, text_height + 10), (self._oled.width, text_height + 10)], fill=255)
        
        # Calculate area for progress bars
        progress_start_y = text_height + 15
        progress_height = 10
        progress_spacing = 5
        label_width = 10
        progress_width = self._oled.width - label_width - 5  # 5 pixels from right edge
        
        # Draw progress bars for each track
        for i, track in enumerate(self._last_state.trks):
            y_pos = progress_start_y + i * (progress_height + progress_spacing)
            
            # Draw track label (first letter of track name)
            track_label = track.name[0]  # First letter of track name
            bbox = self._font.getbbox(track_label)
            self._draw.text(
                (5, y_pos + (progress_height - text_height) // 2),
                track_label,
                font=self._font,
                fill=255
            )
            
            # Draw track progress bar outline
            self._draw.rectangle(
                (label_width + 5, y_pos, label_width + 5 + progress_width, y_pos + progress_height),
                outline=255,
                fill=0
            )
            
            # Draw progress bar segments based on pattern
            segment_width = progress_width / self._last_state.len
            for j, val in enumerate(track.slots):
                if val == 1:  # If this step is active
                    segment_x = label_width + 5 + j * segment_width
                    self._draw.rectangle(
                        (segment_x, y_pos, segment_x + segment_width, y_pos + progress_height),
                        outline=0,
                        fill=255
                    )
            
            # Highlight current position in pattern
            cursor_x = label_width + 5 + self._last_state.division * segment_width
            cursor_height = progress_height + 2
            self._draw.rectangle(
                (cursor_x, y_pos - 1, cursor_x + segment_width, y_pos + cursor_height - 1),
                outline=255,
                fill=None
            )
        
        # Display the image on the OLED
        self._oled.image(self._image)
        self._oled.show()

    def run(self):
        while True:
            if time.monotonic() - self._last_refresh > REFRESH:
                self._last_state = self.receive_state()
                self.receive_input()
                self.render()
                self._last_refresh = time.monotonic()
            
if __name__ == "__main__":
    controller = EmbeddedController()
    try:
        controller.run()
    except KeyboardInterrupt:
        logger.info("Exiting and clearing display...")
        # Clear the display
        controller._oled.fill(0)
        controller._oled.show()
        controller._channel.close()
        logger.info("Display cleared")