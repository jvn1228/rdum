import sys
import time
import numpy as np
from abc import ABC, abstractmethod

IS_LINUX_OS = sys.platform.startswith('linux')

if IS_LINUX_OS:
    import board
    import digitalio
    import adafruit_ssd1306
    from adafruit_blinka.microcontroller.bcm283x.rotaryio import IncrementalEncoder
    import adafruit_mcp3xxx.mcp3008 as MCP
    from adafruit_mcp3xxx.analog_in import AnalogIn
    import busio
    from adafruit_pixelbuf import PixelBuf
    from adafruit_raspberry_pi5_neopixel_write import neopixel_write
else:
    # Mock imports for non-Linux
    class MockBoard:
        D4 = None
        D17 = None
        D18 = None
        D15 = None
        D22 = None
        D23 = None
        D27 = None
        D6 = None
        D12 = None
        SCK = None
        MISO = None
        MOSI = None
        I2C = None
    board = MockBoard()

    class MockDigitalInOut:
        def __init__(self, pin):
            self.value = True
        def switch_to_input(self, pull=None):
            pass
    digitalio = MockDigitalInOut

    class MockSSD1306:
        def __init__(self, width, height, i2c, addr, reset):
            pass
        def fill(self, color):
            pass
        def show(self):
            pass
        def image(self, img):
            pass
    adafruit_ssd1306 = MockSSD1306

    class MockIncrementalEncoder:
        def __init__(self, pin1, pin2):
            self.position = 0
    IncrementalEncoder = MockIncrementalEncoder

    class MockMCP3008:
        def __init__(self, spi, cs):
            pass
    MCP = MockMCP3008

    class MockAnalogIn:
        def __init__(self, mcp, pin):
            self.value = 40000
    AnalogIn = MockAnalogIn

    class MockBusio:
        def SPI(self, clock, miso, mosi):
            pass
    busio = MockBusio

    class MockPixelBuf:
        def __init__(self, size, **kwargs):
            pass
        def __setitem__(self, key, value):
            pass
        def fill(self, color):
            pass
        def show(self):
            pass
    PixelBuf = MockPixelBuf
    neopixel_write = None # Not used in mock

class BaseSwitch(ABC):
    @property
    @abstractmethod
    def value(self) -> bool:
        pass

    @property
    @abstractmethod
    def changed(self) -> bool:
        pass

class BasePad(ABC):
    @property
    @abstractmethod
    def value(self) -> int:
        pass

    @property
    @abstractmethod
    def velocity(self) -> int:
        pass

    @property
    @abstractmethod
    def percent(self) -> int:
        pass

    @property
    @abstractmethod
    def armed(self) -> bool:
        pass

    @abstractmethod
    def zero(self):
        pass

class BasePixelbuf(ABC):
    @abstractmethod
    def __init__(self, pin, size, **kwargs):
        pass

    @abstractmethod
    def __setitem__(self, key, value):
        pass

    @abstractmethod
    def fill(self, color):
        pass

    @abstractmethod
    def show(self):
        pass

class KalmanFilter:
    def __init__(self, F, H, Q, R, x0, P0):
        self._F = F
        self._H = H
        self._Q = Q
        self._R = R
        self.x = x0
        self._P = P0

    def step(self, y):
        xk = self._F @ self.x
        Pk = self._F @ self._P @ self._F.T + self._Q

        K = Pk @ self._H.T @ np.linalg.inv(self._H @ Pk @ self._H.T + self._R)
        self.x = xk + K @ (y - self._H @ xk)
        self._P = Pk - K @ self._H @ Pk

def new_default_KalmanFilter():
    T = 0.01
    H = np.array([[1,0]])
    F = np.array([[1, T], [0, 1]])
    Q = np.array([[T**3/3, T**2/2], [T**2/2, T]])
    R = np.array([[1]])

    return KalmanFilter(
        F=F,
        H=H,
        Q=Q,
        R=R,
        x0=np.array([0, 0]),
        P0=np.array([[1, 0], [0, 1]])
    )

if IS_LINUX_OS:
    class Switch(BaseSwitch):
        def __init__(self, pin: digitalio.DigitalInOut):
            self._pin = pin
            self.last_val = not self._pin.value
            self._changed = False

        @property
        def value(self) -> bool:
            val = not self._pin.value
            if val != self.last_val:
                self._changed = True
                self.last_val = val
            elif self._changed:
                self._changed = False
            return val
        
        @property
        def changed(self) -> bool:
            return self._changed

    class Pad(BasePad):
        def __init__(self, adc: AnalogIn):
            self._adc = adc
            self._offset = 0
            self._max = 40000
            self._min = 4000
            self._trigger_threshold = 15000
            self._kalman = new_default_KalmanFilter()
            self.last_triggered = time.monotonic()
            self._armed = True
        
        @property
        def value(self) -> int:
            if self._adc.value - self._offset < self._trigger_threshold and self._armed:
                self.last_triggered = time.monotonic()
                self._armed = False
            if (time.monotonic() - self.last_triggered) > .02:
                self._armed = True
            return self._adc.value - self._offset
        
        @property
        def velocity(self) -> int:
            return int(self.value / self._max * 127)
        
        @property
        def percent(self) -> int:
            return self.value // self._max

        @property
        def armed(self) -> bool:
            return self._armed

        def zero(self):
            self._offset = self._adc.value

    class Pi5Pixelbuf(PixelBuf, BasePixelbuf):
        def __init__(self, pin, size, **kwargs):
            self._pin = pin
            super().__init__(size=size, **kwargs)

        def _transmit(self, buf):
            neopixel_write(self._pin, buf)
else:
    # Mock implementations for non-Linux
    class Switch(BaseSwitch):
        def __init__(self, digital_in_out):
            self._pin = digital_in_out
            self.last_val = self._pin.value
            self._changed = False

        @property
        def value(self) -> bool:
            val = self._pin.value
            if val != self.last_val:
                self._changed = True
                self.last_val = val
            elif self._changed:
                self._changed = False
            return val
        
        @value.setter
        def value(self, new_value: bool):
            self._pin.value = new_value

        @property
        def changed(self) -> bool:
            return self._changed

    class Pad(BasePad):
        def __init__(self, analog_in):
            self._adc = analog_in
            self._offset = 0
            self._max = 40000
            self._min = 4000
            self._trigger_threshold = 11000
            self.last_triggered = time.monotonic()
            self._armed = False
            self._press_start_time = 0
            self._decay_duration = 1.0 # Time in seconds for value to decay from max to 0
            self._decay_duration = 1.0 # Time in seconds for value to decay from max to 0

        @property
        def value(self) -> int:
            if self._press_start_time > 0:
                hold_duration = time.monotonic() - self._press_start_time
                # Calculate decay factor: 1.0 at start, decreases to 0 over _decay_duration
                decay_factor = 1.0 - min(1.0, hold_duration / self._decay_duration)
                simulated_value = int(self._max * decay_factor)
                return simulated_value
            return self._max
        
        @value.setter
        def value(self, new_value: int):
            if new_value < self._max and not self._armed:
                self.last_triggered = time.monotonic()
                self._armed = True
                self._press_start_time = time.monotonic()
            elif new_value == self._max:
                self._press_start_time = 0
                self._armed = False

        @property
        def velocity(self) -> int:
            return int(self.value / self._max * 127)
        
        @property
        def percent(self) -> int:
            return self.value // self._max

        @property
        def armed(self) -> bool:
            return self._armed
        
        @armed.setter
        def armed(self, val: bool):
            self._armed = val

        def zero(self):
            self._offset = self._adc.value

    class Pi5Pixelbuf(BasePixelbuf):
        def __init__(self, pin, size, auto_write=False, byteorder="BGR"):
            self._pixels = [(0,0,0)] * size
            self._auto_write = auto_write
            self._byteorder = byteorder
            self._size = size

        def __setitem__(self, key, value):
            if isinstance(key, slice):
                start, stop, step = key.indices(self._size)
                for i in range(start, stop, step):
                    self._pixels[i] = value
            else:
                self._pixels[key] = value
            if self._auto_write:
                self.show()

        def fill(self, color):
            for i in range(self._size):
                self._pixels[i] = color
            if self._auto_write:
                self.show()

        def show(self):
            # In a mock environment, this would typically print or log the pixel state
            # For now, we'll just pass
            pass