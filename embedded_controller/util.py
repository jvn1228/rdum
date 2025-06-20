import socket
from adafruit_mcp3xxx.analog_in import AnalogIn
from adafruit_pixelbuf import PixelBuf
from adafruit_raspberry_pi5_neopixel_write import neopixel_write
import numpy as np
import time
import digitalio

def get_ip():
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.settimeout(0)
    try:
        # doesn't even have to be reachable
        s.connect(('10.254.254.254', 1))
        IP = s.getsockname()[0]
    except Exception:
        IP = '127.0.0.1'
    finally:
        s.close()
    return IP

class Switch:
    def __init__(self, pin: digitalio.DigitalInOut):
        self._pin = pin
        self.last_val = not self._pin.value
        self.changed = False

    @property
    def value(self) -> bool:
        val = not self._pin.value
        if val != self.last_val:
            self.changed = True
            self.last_val = val
        elif self.changed:
            self.changed = False
        return val

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
        self._offset = 0#adc.value
        # the adc library returns a 16-bit integer even though
        # precision of the adc is only 10 bits
        self._max = 40000
        self._min = 4000
        self._trigger_threshold = 15000
        self._kalman = new_default_KalmanFilter()
        self.last_triggered = time.monotonic()
        self.armed = True
    
    @property
    def value(self) -> int:
        # self._kalman.step(self._adc.value - self._offset)
        # return min(max(0, int(self._kalman.x[1])), self._max)
        if self._adc.value - self._offset < self._trigger_threshold and self.armed:
            self.last_triggered = time.monotonic()
            self.armed = False
        if (time.monotonic() - self.last_triggered) > .02:
            self.armed = True
        return self._adc.value - self._offset
    
    @property
    def velocity(self) -> int:
        return int(self.value / self._max * 127)
    
    @property
    def percent(self) -> int:
        return self.value // self._max

    def zero(self):
        self._offset = self._adc.value

class Pi5Pixelbuf(PixelBuf):
    def __init__(self, pin, size, **kwargs):
        self._pin = pin
        super().__init__(size=size, **kwargs)

    def _transmit(self, buf):
        neopixel_write(self._pin, buf)

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