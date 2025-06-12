import socket
from adafruit_mcp3xxx.analog_in import AnalogIn

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