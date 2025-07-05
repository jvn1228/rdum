import threading
import time
from pynput import keyboard

from hardware import Pad, Switch
from hardware import MockDigitalInOut, MockIncrementalEncoder, MockAnalogIn


class KeyboardInputManager:
    def __init__(self):
        self.main_encoder = MockIncrementalEncoder(None, None)
        self.sub_encoder = MockIncrementalEncoder(None, None)
        self.main_button = MockDigitalInOut(None)
        self.sub_button = MockDigitalInOut(None)
        self.switch_pin = MockDigitalInOut(None)
        self.switch = Switch(self.switch_pin) # Use the Switch from hardware.py

        self.pad_analog_ins = [MockAnalogIn(None, None) for _ in range(8)]
        self.pads = [Pad(ai) for ai in self.pad_analog_ins] # Use the Pad from hardware.py

        self._listener = keyboard.Listener(
            on_press=self._on_press,
            on_release=self._on_release
        )
        self._listener_thread = threading.Thread(target=self._listener.start, daemon=True)
        self._listener_thread.start()

        self._key_map = {
            keyboard.Key.up: lambda: self._change_encoder(self.main_encoder, 1),
            keyboard.Key.down: lambda: self._change_encoder(self.main_encoder, -1),
            keyboard.Key.left: lambda: self._change_encoder(self.sub_encoder, -1),
            keyboard.Key.right: lambda: self._change_encoder(self.sub_encoder, 1),
            keyboard.Key.space: lambda: self._set_button_state(self.main_button, False), # Pressed
            keyboard.Key.enter: lambda: self._set_button_state(self.sub_button, False), # Pressed
            keyboard.KeyCode.from_char('s'): lambda: self._toggle_switch(),
            keyboard.KeyCode.from_char('q'): lambda: self._set_pad_state(0, True),
            keyboard.KeyCode.from_char('w'): lambda: self._set_pad_state(1, True),
            keyboard.KeyCode.from_char('e'): lambda: self._set_pad_state(2, True),
            keyboard.KeyCode.from_char('r'): lambda: self._set_pad_state(3, True),
            keyboard.KeyCode.from_char('t'): lambda: self._set_pad_state(4, True),
            keyboard.KeyCode.from_char('y'): lambda: self._set_pad_state(5, True),
            keyboard.KeyCode.from_char('u'): lambda: self._set_pad_state(6, True),
            keyboard.KeyCode.from_char('i'): lambda: self._set_pad_state(7, True),
        }

        self._release_key_map = {
            keyboard.Key.space: lambda: self._set_button_state(self.main_button, True), # Released
            keyboard.Key.enter: lambda: self._set_button_state(self.sub_button, True), # Released
            keyboard.KeyCode.from_char('q'): lambda: self._set_pad_state(0, False),
            keyboard.KeyCode.from_char('w'): lambda: self._set_pad_state(1, False),
            keyboard.KeyCode.from_char('e'): lambda: self._set_pad_state(2, False),
            keyboard.KeyCode.from_char('r'): lambda: self._set_pad_state(3, False),
            keyboard.KeyCode.from_char('t'): lambda: self._set_pad_state(4, False),
            keyboard.KeyCode.from_char('y'): lambda: self._set_pad_state(5, False),
            keyboard.KeyCode.from_char('u'): lambda: self._set_pad_state(6, False),
            keyboard.KeyCode.from_char('i'): lambda: self._set_pad_state(7, False),
        }

    def _change_encoder(self, encoder: MockIncrementalEncoder, delta: int):
        encoder.position += delta

    def _set_button_state(self, button: MockDigitalInOut, is_released: bool):
        button.value = is_released # True for released, False for pressed

    def _toggle_switch(self):
        self.switch.value = not self.switch.value

    def _set_pad_state(self, pad_idx: int, is_pressed: bool):
        if 0 <= pad_idx < len(self.pads):
            if is_pressed:
                self.pads[pad_idx].value = 0 # Any non-zero value to indicate pressed
            else:
                self.pads[pad_idx].value = 40000 # Zero to indicate released

    def _on_press(self, key):
        action = self._key_map.get(key)
        if action:
            action()

    def _on_release(self, key):
        action = self._release_key_map.get(key)
        if action:
            action()

    def stop(self):
        self._listener.stop()
        self._listener_thread.join()

if __name__ == '__main__':
    # Example usage for testing
    print("Keyboard Input Manager Test. Press keys:")
    print("Up/Down: Main Encoder, Left/Right: Sub Encoder")
    print("Space: Main Button, Enter: Sub Button, S: Switch")
    print("Q,W,E,R,T,Y,U,I: Pads (hold for velocity)")

    kb_manager = KeyboardInputManager()

    try:
        while True:
            # You would typically integrate this into your main application loop
            # For testing, just print states
            print(f"Enc1: {kb_manager.main_encoder.position}, Enc2: {kb_manager.sub_encoder.position}, "
                  f"Btn1: {not kb_manager.main_button.value}, Btn2: {not kb_manager.sub_button.value}, "
                  f"Switch: {kb_manager.switch.value}, Pads: {[pad.value for pad in kb_manager.pads]}")
            time.sleep(0.1)
    except KeyboardInterrupt:
        print("Stopping keyboard input manager.")
        kb_manager.stop()